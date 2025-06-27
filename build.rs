//! Build script for RUITL projects
//!
//! This build script automatically compiles .ruitl template files into Rust code
//! during the cargo build process. It integrates seamlessly with Cargo's dependency
//! tracking to ensure templates are recompiled when changed.

use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    // Get environment variables
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let src_dir = Path::new(&manifest_dir).join("src");
    let templates_dir = src_dir.join("templates");
    let generated_dir = Path::new(&out_dir).join("generated");

    println!("cargo:rerun-if-changed=src/templates");
    println!("cargo:rerun-if-changed=templates");

    // Create generated directory
    if let Err(e) = fs::create_dir_all(&generated_dir) {
        eprintln!("Failed to create generated directory: {}", e);
        process::exit(1);
    }

    // Find and compile .ruitl files
    let mut compiled_count = 0;
    let mut errors = Vec::new();

    // Check both src/templates and templates directories
    let template_dirs = vec![templates_dir, Path::new(&manifest_dir).join("templates")];

    for template_dir in template_dirs {
        if !template_dir.exists() {
            continue;
        }

        match compile_templates_in_dir(&template_dir, &generated_dir) {
            Ok(count) => compiled_count += count,
            Err(e) => errors.push(format!("Error in {}: {}", template_dir.display(), e)),
        }
    }

    // Report results
    if !errors.is_empty() {
        eprintln!("RUITL template compilation failed:");
        for error in errors {
            eprintln!("  {}", error);
        }
        process::exit(1);
    }

    if compiled_count > 0 {
        println!("cargo:warning=Compiled {} RUITL templates", compiled_count);

        // Generate mod.rs file for easy imports
        if let Err(e) = generate_module_file(&generated_dir) {
            eprintln!("Failed to generate module file: {}", e);
            process::exit(1);
        }

        // Tell Cargo where to find the generated code
        println!(
            "cargo:rustc-env=RUITL_GENERATED_DIR={}",
            generated_dir.display()
        );
    }
}

fn compile_templates_in_dir(
    template_dir: &Path,
    output_dir: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut compiled_count = 0;

    // Walk through all .ruitl files
    for entry in walkdir::WalkDir::new(template_dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ruitl") {
            println!("cargo:rerun-if-changed={}", path.display());

            if let Err(e) = compile_template_file(path, output_dir) {
                return Err(format!("Failed to compile {}: {}", path.display(), e).into());
            }

            compiled_count += 1;
        }
    }

    Ok(compiled_count)
}

fn compile_template_file(
    template_path: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read template file
    let content = fs::read_to_string(template_path)?;

    // Parse template - using a simplified parser since we can't import ruitl modules in build.rs
    // In a real implementation, this would use the full RuitlParser
    let generated_code = generate_rust_code_from_template(&content, template_path)?;

    // Determine output file path
    let template_name = template_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid template filename")?;

    let output_file = output_dir.join(format!("{}.rs", template_name.to_lowercase()));

    // Create parent directories if needed
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write generated code
    fs::write(&output_file, generated_code)?;

    Ok(())
}

fn generate_rust_code_from_template(
    content: &str,
    template_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_name = template_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid template filename")?;

    // Parse the template content
    let parsed_template = parse_simple_template(content)?;

    // Extract component name and basic structure
    let component_name = to_pascal_case(template_name);
    let props_name = format!("{}Props", component_name);

    // Generate props struct based on component definition
    let props_fields = if let Some(component_def) = &parsed_template.component {
        generate_props_fields(&component_def.props)
    } else {
        "// No props defined\n    _phantom: std::marker::PhantomData<()>,".to_string()
    };

    // Generate render body based on template definition
    let render_body = if let Some(template_def) = &parsed_template.template {
        generate_render_body(&template_def.body)
    } else {
        r#"Ok(html! {
            <div class="placeholder">
                <p>Template not found</p>
            </div>
        })"#
        .to_string()
    };

    let generated_code = format!(
        r#"// Generated from {template_path}
// This file is automatically generated by RUITL build script
// DO NOT EDIT MANUALLY

use ruitl::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {props_name} {{
{props_fields}
}}

impl ComponentProps for {props_name} {{
    fn validate(&self) -> ruitl::error::Result<()> {{
        Ok(())
    }}
}}

#[derive(Debug)]
pub struct {component_name};

impl Component for {component_name} {{
    type Props = {props_name};

    fn render(&self, props: &Self::Props, context: &ComponentContext) -> ruitl::error::Result<Html> {{
        {render_body}
    }}
}}

// Re-export for convenience
pub use {component_name} as {template_name}Component;
"#,
        template_path = template_path.display(),
        component_name = component_name,
        props_name = props_name,
        template_name = template_name,
        props_fields = props_fields,
        render_body = render_body
    );

    Ok(generated_code)
}

// Simple template parser for build script
#[derive(Debug)]
struct SimpleParsedTemplate {
    component: Option<SimpleComponentDef>,
    template: Option<SimpleTemplateDef>,
}

#[derive(Debug)]
struct SimpleComponentDef {
    name: String,
    props: Vec<SimplePropDef>,
}

#[derive(Debug)]
struct SimplePropDef {
    name: String,
    prop_type: String,
    default_value: Option<String>,
}

#[derive(Debug)]
struct SimpleTemplateDef {
    name: String,
    params: Vec<SimpleParamDef>,
    body: String, // For now, just store as string
}

#[derive(Debug)]
struct SimpleParamDef {
    name: String,
    param_type: String,
}

fn parse_simple_template(
    content: &str,
) -> Result<SimpleParsedTemplate, Box<dyn std::error::Error>> {
    let mut result = SimpleParsedTemplate {
        component: None,
        template: None,
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("component ") {
            let (component_def, next_i) = parse_component_block(&lines, i)?;
            result.component = Some(component_def);
            i = next_i;
        } else if line.starts_with("ruitl ") {
            let (template_def, next_i) = parse_template_block(&lines, i)?;
            result.template = Some(template_def);
            i = next_i;
        } else {
            i += 1;
        }
    }

    Ok(result)
}

fn parse_component_block(
    lines: &[&str],
    start: usize,
) -> Result<(SimpleComponentDef, usize), Box<dyn std::error::Error>> {
    let line = lines[start].trim();
    let name = line
        .strip_prefix("component ")
        .and_then(|s| s.strip_suffix(" {"))
        .ok_or("Invalid component definition")?
        .trim()
        .to_string();

    let mut i = start + 1;
    let mut props = Vec::new();

    // Look for props block
    while i < lines.len() {
        let line = lines[i].trim();
        if line == "props {" {
            i += 1;
            // Parse props
            while i < lines.len() {
                let line = lines[i].trim();
                if line == "}" {
                    break;
                }
                if !line.is_empty() && !line.starts_with("//") {
                    if let Some(prop) = parse_prop_line(line) {
                        props.push(prop);
                    }
                }
                i += 1;
            }
        } else if line == "}" {
            break;
        }
        i += 1;
    }

    Ok((SimpleComponentDef { name, props }, i + 1))
}

fn parse_template_block(
    lines: &[&str],
    start: usize,
) -> Result<(SimpleTemplateDef, usize), Box<dyn std::error::Error>> {
    let line = lines[start].trim();

    // Parse "ruitl TemplateName(param1: Type1, param2: Type2) {"
    let ruitl_part = line
        .strip_prefix("ruitl ")
        .ok_or("Invalid template definition")?;
    let paren_pos = ruitl_part.find('(').ok_or("Missing parameter list")?;
    let name = ruitl_part[..paren_pos].trim().to_string();

    // Parse parameters (simplified)
    let params_str = ruitl_part[paren_pos + 1..]
        .strip_suffix(") {")
        .ok_or("Invalid parameter list")?;
    let mut params = Vec::new();

    if !params_str.trim().is_empty() {
        for param in params_str.split(',') {
            let param = param.trim();
            let parts: Vec<&str> = param.split(':').collect();
            if parts.len() == 2 {
                params.push(SimpleParamDef {
                    name: parts[0].trim().to_string(),
                    param_type: parts[1].trim().to_string(),
                });
            }
        }
    }

    // For now, just store the body as a string
    let mut i = start + 1;
    let mut body_lines = Vec::new();
    let mut brace_count = 1;

    while i < lines.len() && brace_count > 0 {
        let line = lines[i];
        for c in line.chars() {
            match c {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                _ => {}
            }
        }
        if brace_count > 0 {
            body_lines.push(line);
        }
        i += 1;
    }

    let body = body_lines.join("\n");

    Ok((SimpleTemplateDef { name, params, body }, i))
}

fn parse_prop_line(line: &str) -> Option<SimplePropDef> {
    let line = line.trim_end_matches(',');

    if let Some(eq_pos) = line.find('=') {
        // Has default value
        let (prop_part, default_part) = line.split_at(eq_pos);
        let default_value = default_part[1..].trim().to_string();

        let parts: Vec<&str> = prop_part.split(':').collect();
        if parts.len() == 2 {
            return Some(SimplePropDef {
                name: parts[0].trim().to_string(),
                prop_type: parts[1].trim().to_string(),
                default_value: Some(default_value),
            });
        }
    } else {
        // No default value
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 2 {
            return Some(SimplePropDef {
                name: parts[0].trim().to_string(),
                prop_type: parts[1].trim().to_string(),
                default_value: None,
            });
        }
    }

    None
}

fn generate_props_fields(props: &[SimplePropDef]) -> String {
    if props.is_empty() {
        return "    // No props defined\n    _phantom: std::marker::PhantomData<()>,".to_string();
    }

    props
        .iter()
        .map(|prop| {
            let default_comment = if prop.default_value.is_some() {
                format!(" // default: {}", prop.default_value.as_ref().unwrap())
            } else {
                String::new()
            };
            format!(
                "    pub {}: {},{}",
                prop.name, prop.prop_type, default_comment
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn generate_render_body(body: &str) -> String {
    // Very simplified template body generation
    // For now, just generate a basic HTML structure
    let simple_body = body.trim();

    if simple_body.contains("<") {
        // Looks like HTML, try to convert to html! macro
        let html_content = simple_body.replace("{", "{ ").replace("}", " }");

        format!(
            r#"Ok(html! {{
            {}
        }})"#,
            html_content
        )
    } else {
        // Fallback to simple text
        format!(
            r#"Ok(html! {{
            <div class="template">
                <p>{}</p>
            </div>
        }})"#,
            simple_body
        )
    }
}

fn generate_module_file(generated_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut module_exports = Vec::new();
    let mut pub_uses = Vec::new();

    // Find all generated .rs files
    for entry in fs::read_dir(generated_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem != "mod" {
                    module_exports.push(format!("pub mod {};", stem));
                    pub_uses.push(format!("pub use {}::*;", stem));
                }
            }
        }
    }

    let mod_content = format!(
        r#"// Generated module file for RUITL templates
// This file is automatically generated by RUITL build script
// DO NOT EDIT MANUALLY

{}

{}
"#,
        module_exports.join("\n"),
        pub_uses.join("\n")
    );

    fs::write(generated_dir.join("mod.rs"), mod_content)?;

    Ok(())
}

fn to_pascal_case(s: &str) -> String {
    s.split(&['_', '-'][..])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

// Add walkdir as a build dependency if not already present
// walkdir is included as a build dependency

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("user-card"), "UserCard");
        assert_eq!(to_pascal_case("button"), "Button");
        assert_eq!(to_pascal_case("my_awesome_component"), "MyAwesomeComponent");
    }

    #[test]
    fn test_template_generation() {
        let content = "component Button { props { text: String } }";
        let path = Path::new("Button.ruitl");

        let result = generate_rust_code_from_template(content, path);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("struct ButtonProps"));
        assert!(code.contains("struct Button"));
        assert!(code.contains("impl Component for Button"));
    }
}
