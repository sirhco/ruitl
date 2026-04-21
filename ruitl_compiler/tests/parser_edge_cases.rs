//! Edge-case tests for the RUITL parser.
//!
//! These complement the in-module unit tests by exercising paths that are
//! easy to overlook — DOCTYPE, string escapes, comments inside templates,
//! nested component composition, unusual match patterns, and malformed
//! inputs. They target both the happy path (parse succeeds with the right
//! AST shape) and the error path (parse fails with the expected message).

use ruitl_compiler::{parse_str, RuitlFile, TemplateAst};

fn parse_ok(source: &str) -> RuitlFile {
    parse_str(source).unwrap_or_else(|e| panic!("expected parse success, got:\n{}", e))
}

fn parse_err(source: &str) -> String {
    let e = parse_str(source).expect_err("expected parse to fail");
    format!("{}", e)
}

// ---------------------------------------------------------------------------
// DOCTYPE
// ---------------------------------------------------------------------------

#[test]
fn parses_doctype_html5() {
    let src = r#"
component Page {
    props { title: String }
}

ruitl Page(title: String) {
    <!DOCTYPE html>
    <html><head><title>{title}</title></head><body></body></html>
}
"#;
    let file = parse_ok(src);
    let body = &file.templates[0].body;
    let TemplateAst::Fragment(nodes) = body else {
        panic!("expected Fragment at root, got {:?}", body);
    };
    assert!(
        matches!(&nodes[0], TemplateAst::Text(t) if t.contains("<!DOCTYPE")),
        "first child should be the DOCTYPE text"
    );
}

// ---------------------------------------------------------------------------
// Comments inside templates
// ---------------------------------------------------------------------------

#[test]
fn parses_line_comments_before_and_after_blocks() {
    let src = r#"
// top-level comment
component Foo {
    // inside component
    props {
        // inside props
        x: String,
    }
}

// between defs
ruitl Foo(x: String) {
    <div>{x}</div>
}
"#;
    let file = parse_ok(src);
    assert_eq!(file.components.len(), 1);
    assert_eq!(file.templates.len(), 1);
}

#[test]
fn parses_block_comments() {
    let src = r#"
/* license header
   multi-line */
component Foo {
    props { x: String }
}
ruitl Foo(x: String) { <p>{x}</p> }
"#;
    parse_ok(src);
}

// ---------------------------------------------------------------------------
// Nested component composition
// ---------------------------------------------------------------------------

#[test]
fn parses_nested_at_component_invocation() {
    let src = r#"
component Inner {
    props { label: String }
}
ruitl Inner(label: String) { <span>{label}</span> }

component Outer {
    props { label: String }
}
ruitl Outer(label: String) {
    <div>
        @Inner(label: label.clone())
    </div>
}
"#;
    parse_ok(src);
}

// ---------------------------------------------------------------------------
// Match patterns
// ---------------------------------------------------------------------------

#[test]
fn parses_match_with_enum_variant_patterns() {
    let src = r#"
component Choice {
    props { value: String }
}

ruitl Choice(value: String) {
    <span>
        match value.as_str() {
            "a" => { <em>a</em> }
            "b" => { <strong>b</strong> }
            _ => { <span>other</span> }
        }
    </span>
}
"#;
    parse_ok(src);
}

// ---------------------------------------------------------------------------
// Malformed inputs
// ---------------------------------------------------------------------------

#[test]
fn rejects_missing_closing_brace_on_component() {
    let err = parse_err("component Foo { props { text: String }");
    assert!(
        err.contains("Expected") || err.contains("line"),
        "error should mention what was expected: {}",
        err
    );
}

#[test]
fn rejects_bad_prop_syntax_missing_colon() {
    let err = parse_err(
        "component Foo { props { text String } } ruitl Foo(text: String) { <p>{text}</p> }",
    );
    assert!(err.contains("Expected ':'"), "error missing ':': {}", err);
}

#[test]
fn rejects_unclosed_element() {
    let err = parse_err("component Foo { props { } } ruitl Foo() { <button>click me }");
    assert!(
        err.contains("Expected closing tag") || err.contains("line"),
        "error should mention closing tag: {}",
        err
    );
}

#[test]
fn rejects_unclosed_expression() {
    let err = parse_err(
        "component Foo { props { x: String } } ruitl Foo(x: String) { <p>{x</p> }",
    );
    assert!(
        err.contains("'") || err.contains("expression") || err.contains("closing"),
        "error should flag the unclosed expression: {}",
        err
    );
}

#[test]
fn rejects_lifetime_generics() {
    let err = parse_err("component Foo<'a> { props { x: String } }");
    assert!(
        err.contains("Lifetime parameters"),
        "error should mention lifetime rejection: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Hyphenated and namespaced attribute names
// ---------------------------------------------------------------------------

#[test]
fn parses_raw_expression_marker() {
    let src = r#"
component Dump {
    props { html: String }
}
ruitl Dump(html: String) {
    <div>{!html}</div>
}
"#;
    let file = parse_ok(src);
    // Walk the body to find the RawExpression node.
    fn find_raw(ast: &TemplateAst) -> bool {
        match ast {
            TemplateAst::RawExpression(_) => true,
            TemplateAst::Element { children, .. } => children.iter().any(find_raw),
            TemplateAst::Fragment(ns) => ns.iter().any(find_raw),
            _ => false,
        }
    }
    assert!(find_raw(&file.templates[0].body));
}

#[test]
fn parses_hyphenated_and_namespaced_attribute_names() {
    let src = r#"
component Thing {
    props { }
}
ruitl Thing() {
    <svg xmlns:xlink="http://www.w3.org/1999/xlink" aria-hidden="true" data-testid="svg"/>
}
"#;
    parse_ok(src);
}
