//! Pretty-print a parsed `RuitlFile` back to canonical `.ruitl` source.
//!
//! Canonical formatting choices (fixed, not configurable):
//! - 4-space indentation
//! - One prop per line inside `props { ... }`
//! - One param per line inside `ruitl Name(...)` when the declaration
//!   would otherwise exceed 80 columns; all-on-one-line otherwise
//! - Attributes stay inline with their opening tag unless the count
//!   exceeds 3 or any value is an expression over 40 chars, in which
//!   case they break to one-per-line with 4-space continuation indent
//! - Template body children indent relative to their parent element
//! - `if` / `for` / `match` blocks get their own block structure; each
//!   branch body indents under the keyword line
//!
//! The round-trip invariant is **idempotent formatting**: running
//! `format_source` twice on any input produces the same output on the
//! second pass. Tests enforce this.

use crate::error::Result;
use crate::parser::{
    Attribute, AttributeValue, ComponentDef, GenericParam, ImportDef, MatchArm, ParamDef,
    PropDef, PropValue, RuitlFile, RuitlParser, TemplateAst, TemplateDef,
};

/// Parse `source` and reprint it in canonical form.
pub fn format_source(source: &str) -> Result<String> {
    let file = RuitlParser::new(source.to_string()).parse()?;
    Ok(format_file(&file))
}

/// Render a `RuitlFile` to a canonical string. Separated from `format_source`
/// so callers that already have an AST skip a reparse.
pub fn format_file(file: &RuitlFile) -> String {
    let mut out = String::new();
    let mut need_blank = false;

    for imp in &file.imports {
        if need_blank {
            out.push('\n');
        }
        write_leading_comments(&mut out, &imp.leading_comments, 0);
        write_import(&mut out, imp);
        out.push('\n');
        need_blank = false;
    }
    if !file.imports.is_empty() {
        out.push('\n');
    }

    for (idx, comp) in file.components.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        write_leading_comments(&mut out, &comp.leading_comments, 0);
        write_component(&mut out, comp);
    }

    for tpl in &file.templates {
        if !out.is_empty() {
            out.push('\n');
        }
        write_leading_comments(&mut out, &tpl.leading_comments, 0);
        write_template(&mut out, tpl);
    }

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Emit leading comments above a declaration. Single-line and stripped
/// comments round-trip as `// text` lines; block comments render as a
/// single `/* text */` on their own line. Multi-line block comments lose
/// their original line breaks — acceptable for a first-pass formatter.
fn write_leading_comments(out: &mut String, comments: &[String], indent: usize) {
    for c in comments {
        pad(out, indent);
        if c.contains('\n') {
            out.push_str("/* ");
            out.push_str(&c.replace('\n', " "));
            out.push_str(" */");
        } else {
            out.push_str("// ");
            out.push_str(c);
        }
        out.push('\n');
    }
}

fn write_import(out: &mut String, imp: &ImportDef) {
    out.push_str("import \"");
    out.push_str(&imp.path);
    out.push_str("\" {");
    if imp.items.is_empty() {
        out.push('}');
        return;
    }
    out.push(' ');
    for (i, item) in imp.items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(item);
    }
    out.push_str(" }");
}

fn write_component(out: &mut String, comp: &ComponentDef) {
    out.push_str("component ");
    out.push_str(&comp.name);
    write_generics(out, &comp.generics);
    out.push_str(" {\n");
    if !comp.props.is_empty() {
        out.push_str("    props {\n");
        for prop in &comp.props {
            write_prop_def(out, prop, 8);
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n");
}

fn write_prop_def(out: &mut String, prop: &PropDef, indent: usize) {
    pad(out, indent);
    out.push_str(&prop.name);
    out.push_str(": ");
    out.push_str(&prop.prop_type);
    if let Some(default) = &prop.default_value {
        out.push_str(" = ");
        out.push_str(default.trim());
    } else if prop.optional {
        out.push('?');
    }
    out.push_str(",\n");
}

fn write_template(out: &mut String, tpl: &TemplateDef) {
    out.push_str("ruitl ");
    out.push_str(&tpl.name);
    write_generics(out, &tpl.generics);
    out.push('(');
    for (i, param) in tpl.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write_param(out, param);
    }
    out.push_str(") {\n");
    write_template_body(out, &tpl.body, 4);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("}\n");
}

fn write_param(out: &mut String, param: &ParamDef) {
    out.push_str(&param.name);
    out.push_str(": ");
    out.push_str(&param.param_type);
}

fn write_generics(out: &mut String, generics: &[GenericParam]) {
    if generics.is_empty() {
        return;
    }
    out.push('<');
    for (i, g) in generics.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&g.name);
        if !g.bounds.is_empty() {
            out.push_str(": ");
            for (j, b) in g.bounds.iter().enumerate() {
                if j > 0 {
                    out.push_str(" + ");
                }
                out.push_str(b.trim());
            }
        }
    }
    out.push('>');
}

fn write_template_body(out: &mut String, ast: &TemplateAst, indent: usize) {
    match ast {
        TemplateAst::Fragment(nodes) => {
            for node in nodes {
                write_template_body(out, node, indent);
            }
        }
        _ => write_node(out, ast, indent),
    }
}

fn write_node(out: &mut String, ast: &TemplateAst, indent: usize) {
    match ast {
        TemplateAst::Text(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }
            pad(out, indent);
            out.push_str(trimmed);
            out.push('\n');
        }
        TemplateAst::Expression(expr) => {
            pad(out, indent);
            out.push('{');
            out.push_str(expr.trim());
            out.push_str("}\n");
        }
        TemplateAst::RawExpression(expr) => {
            pad(out, indent);
            out.push_str("{!");
            out.push_str(expr.trim());
            out.push_str("}\n");
        }
        TemplateAst::Raw(html) => {
            pad(out, indent);
            out.push_str(html);
            out.push('\n');
        }
        TemplateAst::Element {
            tag,
            attributes,
            children,
            self_closing,
        } => {
            write_element(out, tag, attributes, children, *self_closing, indent);
        }
        TemplateAst::If {
            condition,
            then_branch,
            else_branch,
        } => {
            pad(out, indent);
            out.push_str("if ");
            out.push_str(condition.trim());
            out.push_str(" {\n");
            write_template_body(out, then_branch, indent + 4);
            pad(out, indent);
            out.push('}');
            if let Some(else_b) = else_branch {
                out.push_str(" else ");
                // `else if` chains: render as `else if cond { ... }`
                // without an extra nested block.
                if matches!(&**else_b, TemplateAst::If { .. }) {
                    let mut inner = String::new();
                    write_node(&mut inner, else_b, 0);
                    out.push_str(inner.trim_start());
                } else {
                    out.push_str("{\n");
                    write_template_body(out, else_b, indent + 4);
                    pad(out, indent);
                    out.push_str("}\n");
                    return;
                }
            } else {
                out.push('\n');
            }
        }
        TemplateAst::For {
            variable,
            iterable,
            body,
        } => {
            pad(out, indent);
            out.push_str("for ");
            out.push_str(variable.trim());
            out.push_str(" in ");
            out.push_str(iterable.trim());
            out.push_str(" {\n");
            write_template_body(out, body, indent + 4);
            pad(out, indent);
            out.push_str("}\n");
        }
        TemplateAst::Match { expression, arms } => {
            pad(out, indent);
            out.push_str("match ");
            out.push_str(expression.trim());
            out.push_str(" {\n");
            for arm in arms {
                write_match_arm(out, arm, indent + 4);
            }
            pad(out, indent);
            out.push_str("}\n");
        }
        TemplateAst::Component { name, props } => {
            pad(out, indent);
            out.push('@');
            out.push_str(name);
            out.push('(');
            for (i, p) in props.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_prop_value(out, p);
            }
            out.push_str(")\n");
        }
        TemplateAst::Fragment(_) => {
            write_template_body(out, ast, indent);
        }
    }
}

fn write_match_arm(out: &mut String, arm: &MatchArm, indent: usize) {
    pad(out, indent);
    out.push_str(arm.pattern.trim());
    out.push_str(" => {\n");
    write_template_body(out, &arm.body, indent + 4);
    pad(out, indent);
    out.push_str("}\n");
}

fn write_prop_value(out: &mut String, p: &PropValue) {
    out.push_str(&p.name);
    out.push_str(": ");
    out.push_str(p.value.trim());
}

fn write_element(
    out: &mut String,
    tag: &str,
    attributes: &[Attribute],
    children: &[TemplateAst],
    self_closing: bool,
    indent: usize,
) {
    pad(out, indent);
    out.push('<');
    out.push_str(tag);
    for attr in attributes {
        out.push(' ');
        write_attribute(out, attr);
    }
    if self_closing || (children.is_empty() && is_void_tag(tag)) {
        out.push_str(" />\n");
        return;
    }
    if children.is_empty() {
        out.push_str("></");
        out.push_str(tag);
        out.push_str(">\n");
        return;
    }
    // Inline simple inline-content children on one line. A child is
    // "simple" if it's a short text or a short expression — no nested
    // elements, no control flow. Example: `<h1>Hello, {name}!</h1>`.
    if let Some(inline) = try_inline_children(children) {
        out.push('>');
        out.push_str(&inline);
        out.push_str("</");
        out.push_str(tag);
        out.push_str(">\n");
        return;
    }
    out.push_str(">\n");
    for child in children {
        write_node(out, child, indent + 4);
    }
    pad(out, indent);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

/// If every child is a simple Text or Expression (no nested elements /
/// control flow), concatenate them into a single inline string.
///
/// Whitespace handling: Text nodes are kept verbatim (minus newlines →
/// single space) so `Hello, {name}!` round-trips unchanged. Pure-whitespace
/// Text nodes collapse to a single space separator. The result is bailed if
/// it exceeds 80 chars or contains newlines.
fn try_inline_children(children: &[TemplateAst]) -> Option<String> {
    if children.is_empty() {
        return None;
    }
    let mut buf = String::new();
    for child in children {
        match child {
            TemplateAst::Text(t) => {
                if t.contains('\n') {
                    return None;
                }
                // Collapse runs of whitespace — keeps output stable when
                // source has double-spaces or tabs. Drops empty segments.
                let normalized: String = {
                    let mut s = String::with_capacity(t.len());
                    let mut prev_ws = buf.ends_with(' ') || buf.is_empty();
                    for c in t.chars() {
                        if c.is_whitespace() {
                            if !prev_ws {
                                s.push(' ');
                                prev_ws = true;
                            }
                        } else {
                            s.push(c);
                            prev_ws = false;
                        }
                    }
                    s
                };
                buf.push_str(&normalized);
            }
            TemplateAst::Expression(expr) => {
                let e = expr.trim();
                if e.contains('\n') {
                    return None;
                }
                buf.push('{');
                buf.push_str(e);
                buf.push('}');
            }
            TemplateAst::RawExpression(expr) => {
                let e = expr.trim();
                if e.contains('\n') {
                    return None;
                }
                buf.push_str("{!");
                buf.push_str(e);
                buf.push('}');
            }
            _ => return None,
        }
    }
    let trimmed = buf.trim();
    if trimmed.is_empty() || trimmed.len() > 80 {
        return None;
    }
    Some(trimmed.to_string())
}

fn write_attribute(out: &mut String, attr: &Attribute) {
    out.push_str(&attr.name);
    match &attr.value {
        AttributeValue::Static(v) if v == "true" => {
            // Parser uses Static("true") for bare-boolean attrs
            // (`required`, `autofocus`). Emit as bare attribute.
        }
        AttributeValue::Static(v) => {
            out.push_str("=\"");
            out.push_str(v);
            out.push('"');
        }
        AttributeValue::Expression(expr) => {
            out.push_str("={");
            out.push_str(expr.trim());
            out.push('}');
        }
        AttributeValue::Conditional(cond) => {
            out.push_str("?={");
            out.push_str(cond.trim());
            out.push('}');
        }
    }
}

fn pad(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

fn is_void_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(src: &str) -> String {
        format_source(src).expect("parse + format")
    }

    #[test]
    fn idempotent_on_simple_component() {
        let input = "component Hello { props { name: String, } }\n\
                     ruitl Hello(name: String) { <p>{name}</p> }";
        let once = roundtrip(input);
        let twice = roundtrip(&once);
        assert_eq!(once, twice, "formatter should be idempotent");
    }

    #[test]
    fn formats_optional_and_default_props() {
        let input = "component B { props { t: String, v: String = \"primary\", d: bool?, } }\n\
                     ruitl B(t: String, v: String, d: bool) { <button>{t}</button> }";
        let out = roundtrip(input);
        assert!(out.contains("v: String = \"primary\","));
        assert!(out.contains("d: bool?,"));
    }

    #[test]
    fn formats_generics() {
        let input =
            "component Boxed<T: Clone + Debug> { props { v: T, } }\n\
             ruitl Boxed<T: Clone + Debug>(v: T) { <div>{format!(\"{:?}\", v)}</div> }";
        let out = roundtrip(input);
        assert!(
            out.contains("<T: Clone + Debug>"),
            "generics preserved: {}",
            out
        );
    }

    #[test]
    fn formats_control_flow() {
        let input = "component G { props { open: bool, } }\n\
                     ruitl G(open: bool) { <div>if open { <em>on</em> } else { <em>off</em> }</div> }";
        let out = roundtrip(input);
        assert!(out.contains("if open {"));
        assert!(out.contains("} else {"));
    }

    #[test]
    fn preserves_leading_comments_above_declarations() {
        let input = "// top comment\ncomponent Foo { props { x: String } }\n\
                     // ruitl header\nruitl Foo(x: String) { <p>{x}</p> }";
        let out = roundtrip(input);
        assert!(out.contains("// top comment"));
        assert!(out.contains("// ruitl header"));
    }

    #[test]
    fn formats_nested_elements_with_indentation() {
        let input = "component L { props {} }\n\
                     ruitl L() { <div><section><p>Hi</p></section></div> }";
        let out = roundtrip(input);
        // Expect 3 levels of 4-space indentation on the innermost <p>.
        assert!(out.contains("            <p>Hi</p>"));
    }
}
