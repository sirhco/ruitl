//! Simple parser debug tests to isolate parsing issues

use ruitl::parser::RuitlParser;

#[test]
fn test_simple_component_only() {
    let template = r#"
component Button {
    props {
        text: String,
    }
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("SUCCESS: Parsed component-only template");
            println!("Components: {}", ast.components.len());
            println!("Templates: {}", ast.templates.len());
        }
        Err(e) => {
            println!("ERROR parsing component-only: {}", e);
            panic!("Failed to parse simple component");
        }
    }
}

#[test]
fn test_simple_template_only() {
    let template = r#"
templ Button() {
    <button>Click me</button>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("SUCCESS: Parsed template-only");
            println!("Components: {}", ast.components.len());
            println!("Templates: {}", ast.templates.len());
        }
        Err(e) => {
            println!("ERROR parsing template-only: {}", e);
            panic!("Failed to parse simple template: {}", e);
        }
    }
}

#[test]
fn test_template_with_simple_params() {
    let template = r#"
templ Button(text: String) {
    <button>{text}</button>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("SUCCESS: Parsed template with simple params");
            println!("Templates: {}", ast.templates.len());
            if !ast.templates.is_empty() {
                println!("Template name: {}", ast.templates[0].name);
                println!("Template params: {}", ast.templates[0].params.len());
            }
        }
        Err(e) => {
            println!("ERROR parsing template with params: {}", e);
            panic!("Failed to parse template with params: {}", e);
        }
    }
}

#[test]
fn test_template_with_props_type() {
    let template = r#"
templ Button(props: ButtonProps) {
    <button>Click</button>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("SUCCESS: Parsed template with props type");
            println!("Templates: {}", ast.templates.len());
        }
        Err(e) => {
            println!("ERROR parsing template with props type: {}", e);
            println!("This is likely where our issue is!");
        }
    }
}

#[test]
fn test_full_component_and_template() {
    let template = r#"
component Button {
    props {
        text: String,
    }
}

templ Button(props: ButtonProps) {
    <button>{props.text}</button>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("SUCCESS: Parsed full component and template");
            println!("Components: {}", ast.components.len());
            println!("Templates: {}", ast.templates.len());
        }
        Err(e) => {
            println!("ERROR parsing full component and template: {}", e);
        }
    }
}

#[test]
fn test_debug_character_by_character() {
    let template = "templ Button(props: ButtonProps) {";

    let mut parser = RuitlParser::new(template.to_string());

    // Manually step through the parsing to see where it fails
    println!("Starting character-by-character debug");
    println!("Template: '{}'", template);

    // Try to parse just the template signature
    let result = parser.parse();
    match result {
        Ok(_) => println!("Surprisingly succeeded"),
        Err(e) => println!("Failed as expected: {}", e),
    }
}

#[test]
fn test_minimal_failing_case() {
    // This should be the minimal case that reproduces the error
    let template = r#"templ Button(props: ButtonProps) {
    <button>test</button>
}"#;

    println!("Testing minimal failing case:");
    println!("{}", template);

    let mut parser = RuitlParser::new(template.to_string());
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("SUCCESS - minimal case worked!");
            println!("Templates found: {}", ast.templates.len());
        }
        Err(e) => {
            println!("ERROR in minimal case: {}", e);
            println!("This confirms the issue location");
        }
    }
}
