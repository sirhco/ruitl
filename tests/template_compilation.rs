//! Comprehensive tests for RUITL template compilation
//!
//! This test suite covers the complete template compilation pipeline:
//! - Parsing .ruitl files into AST
//! - Generating Rust code from AST
//! - Validating generated code functionality
//! - Testing error handling and edge cases

use ruitl::codegen::CodeGenerator;
use ruitl::parser::{AttributeValue, RuitlParser, TemplateAst};
use ruitl::prelude::*;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_component_compilation() {
    let template = r#"
component Button {
    props {
        text: String,
        disabled: bool = false,
    }
}

ruitl Button(props: ButtonProps) {
    <button disabled?={props.disabled}>
        {props.text}
    </button>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser.parse().expect("Failed to parse template");

    assert_eq!(ast.components.len(), 1);
    assert_eq!(ast.templates.len(), 1);

    let component = &ast.components[0];
    assert_eq!(component.name, "Button");
    assert_eq!(component.props.len(), 2);

    let template_def = &ast.templates[0];
    assert_eq!(template_def.name, "Button");

    // Test code generation
    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("struct ButtonProps"));
    assert!(code_str.contains("struct Button"));
    assert!(code_str.contains("impl Component for Button"));
}

#[test]
fn test_complex_component_with_conditionals_and_loops() {
    let template = r#"
component UserList {
    props {
        users: Vec<User>,
        show_avatars: bool = true,
        title: String = "Users",
    }
}

ruitl UserList(props: UserListProps) {
    <div class="user-list">
        <h2>{props.title}</h2>
        if props.users.is_empty() {
            <p class="empty-message">No users found</p>
        } else {
            <ul class="users">
                for user in props.users {
                    <li class="user-item">
                        if props.show_avatars && user.avatar.is_some() {
                            <img src={user.avatar.unwrap()} alt="Avatar" />
                        }
                        <span class="user-name">{user.name}</span>
                    </li>
                }
            </ul>
        }
    </div>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser.parse().expect("Failed to parse complex template");

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("if props.users.is_empty()"));
    assert!(code_str.contains("into_iter"));
    assert!(code_str.contains("map"));
}

#[test]
fn test_component_composition() {
    let template = r#"
component Card {
    props {
        title: String,
        content: String,
    }
}

component Button {
    props {
        text: String,
        variant: String = "primary",
    }
}

ruitl Card(props: CardProps) {
    <div class="card">
        <h3 class="card-title">{props.title}</h3>
        <p class="card-content">{props.content}</p>
        <div class="card-actions">
            @Button(text: "Read More", variant: "secondary")
            @Button(text: "Share", variant: "outline")
        </div>
    </div>
}

ruitl Button(props: ButtonProps) {
    <button class={format!("btn btn-{}", props.variant)}>
        {props.text}
    </button>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser
        .parse()
        .expect("Failed to parse composition template");

    assert_eq!(ast.components.len(), 2);
    assert_eq!(ast.templates.len(), 2);

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("struct CardProps"));
    assert!(code_str.contains("struct ButtonProps"));
    assert!(code_str.contains("Button"));
    assert!(code_str.contains("Card"));
}

#[test]
fn test_match_expression_compilation() {
    let template = r#"
component StatusBadge {
    props {
        status: String,
    }
}

ruitl StatusBadge(props: StatusBadgeProps) {
    <span class="status-badge">
        match props.status {
            "active" => {
                <span class="status-active">● Active</span>
            }
            "inactive" => {
                <span class="status-inactive">○ Inactive</span>
            }
            "pending" => {
                <span class="status-pending">◐ Pending</span>
            }
            _ => {
                <span class="status-unknown">? Unknown</span>
            }
        }
    </span>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser.parse().expect("Failed to parse match template");

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("match props.status"));
    assert!(code_str.contains("\"active\" =>"));
    assert!(code_str.contains("\"inactive\" =>"));
    assert!(code_str.contains("_ =>"));
}

#[test]
fn test_import_handling() {
    let template = r#"
import "std::collections" { HashMap, Vec }
import "serde" { Serialize, Deserialize }

component DataTable {
    props {
        data: HashMap<String, Vec<String>>,
    }
}

ruitl DataTable(props: DataTableProps) {
    <table class="data-table">
        <tbody>
            for (key, values) in props.data {
                <tr>
                    <td class="key">{key}</td>
                    <td class="values">
                        for value in values {
                            <span class="value">{value}</span>
                        }
                    </td>
                </tr>
            }
        </tbody>
    </table>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser
        .parse()
        .expect("Failed to parse template with imports");

    assert_eq!(ast.imports.len(), 2);
    assert_eq!(ast.imports[0].path, "std::collections");
    assert_eq!(ast.imports[0].items, vec!["HashMap", "Vec"]);

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("use std::collections::{HashMap, Vec}"));
    assert!(code_str.contains("use serde::{Serialize, Deserialize}"));
}

#[test]
fn test_conditional_attributes() {
    let template = r#"
component Input {
    props {
        value: String,
        disabled: bool = false,
        required: bool = false,
        placeholder: String?,
    }
}

ruitl Input(props: InputProps) {
    <input
        type="text"
        value={props.value}
        disabled?={props.disabled}
        required?={props.required}
        placeholder={props.placeholder.as_deref().unwrap_or("")}
        class="form-input"
    />
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser
        .parse()
        .expect("Failed to parse conditional attributes template");

    // Check that conditional attributes are parsed correctly
    let template_def = &ast.templates[0];
    if let TemplateAst::Element { attributes, .. } = &template_def.body {
        let disabled_attr = attributes.iter().find(|a| a.name == "disabled").unwrap();
        assert!(matches!(
            disabled_attr.value,
            AttributeValue::Conditional(_)
        ));

        let required_attr = attributes.iter().find(|a| a.name == "required").unwrap();
        assert!(matches!(
            required_attr.value,
            AttributeValue::Conditional(_)
        ));
    }

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("attr_if"));
}

#[test]
fn test_self_closing_elements() {
    let template = r#"
component Icon {
    props {
        name: String,
        size: String = "medium",
    }
}

ruitl Icon(props: IconProps) {
    <i
        class={format!("icon icon-{} icon-{}", props.name, props.size)}
        aria-hidden="true"
    />
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser
        .parse()
        .expect("Failed to parse self-closing element template");

    let template_def = &ast.templates[0];
    if let TemplateAst::Element { self_closing, .. } = &template_def.body {
        assert!(*self_closing);
    }

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("self_closing"));
}

#[test]
fn test_nested_components() {
    let template = r#"
component Layout {
    props {
        title: String,
        children: Html,
    }
}

component Page {
    props {
        title: String,
        content: String,
    }
}

ruitl Layout(props: LayoutProps) {
    <html>
        <head>
            <title>{props.title}</title>
        </head>
        <body>
            {props.children}
        </body>
    </html>
}

ruitl Page(props: PageProps) {
    @Layout(
        title: props.title.clone(),
        children: html! {
            <main>
                <h1>{props.title}</h1>
                <div class="content">
                    {props.content}
                </div>
            </main>
        }
    )
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser
        .parse()
        .expect("Failed to parse nested components template");

    assert_eq!(ast.components.len(), 2);
    assert_eq!(ast.templates.len(), 2);

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("LayoutProps"));
    assert!(code_str.contains("PageProps"));
}

#[test]
fn test_error_handling_invalid_syntax() {
    let invalid_templates = vec![
        // Missing closing brace
        "component Button { props { text: String }",
        // Invalid prop syntax
        "component Button { props { text String } }",
        // Unclosed element
        "ruitl Button() { <button>Click me }",
        // Invalid expression
        "ruitl Button() { <button>{unclosed_expr</button> }",
    ];

    for template in invalid_templates {
        let mut parser = RuitlParser::new(template.to_string());
        let result = parser.parse();
        assert!(result.is_err(), "Expected error for template: {}", template);
    }
}

#[test]
fn test_complex_expressions() {
    let template = r#"
component Calculator {
    props {
        a: i32,
        b: i32,
        operation: String,
    }
}

ruitl Calculator(props: CalculatorProps) {
    <div class="calculator">
        <div class="expression">
            {props.a} {props.operation} {props.b} =
            {
                match props.operation.as_str() {
                    "+" => props.a + props.b,
                    "-" => props.a - props.b,
                    "*" => props.a * props.b,
                    "/" => if props.b != 0 { props.a / props.b } else { 0 },
                    _ => 0,
                }
            }
        </div>
    </div>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser
        .parse()
        .expect("Failed to parse complex expressions template");

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("props.a"));
    assert!(code_str.contains("props.b"));
    assert!(code_str.contains("props.operation"));
}

#[test]
fn test_file_compilation_workflow() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let templates_dir = temp_dir.path().join("templates");
    let generated_dir = temp_dir.path().join("generated");

    fs::create_dir_all(&templates_dir).expect("Failed to create templates dir");
    fs::create_dir_all(&generated_dir).expect("Failed to create generated dir");

    // Write test template files
    let button_template = r#"
component Button {
    props {
        text: String,
        variant: String = "primary",
    }
}

ruitl Button(props: ButtonProps) {
    <button class={format!("btn btn-{}", props.variant)}>
        {props.text}
    </button>
}
"#;

    let card_template = r#"
component Card {
    props {
        title: String,
        content: String,
    }
}

ruitl Card(props: CardProps) {
    <div class="card">
        <h3>{props.title}</h3>
        <p>{props.content}</p>
        @Button(text: "Action", variant: "secondary")
    </div>
}
"#;

    fs::write(templates_dir.join("Button.ruitl"), button_template)
        .expect("Failed to write Button template");
    fs::write(templates_dir.join("Card.ruitl"), card_template)
        .expect("Failed to write Card template");

    // Simulate compilation process
    let template_files = vec![
        templates_dir.join("Button.ruitl"),
        templates_dir.join("Card.ruitl"),
    ];

    for template_file in template_files {
        let content = fs::read_to_string(&template_file).expect("Failed to read template");
        let mut parser = RuitlParser::new(content);
        let ast = parser
            .parse()
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", template_file.display(), e));

        let mut generator = CodeGenerator::new(ast);
        let generated_code = generator.generate().unwrap_or_else(|e| {
            panic!(
                "Failed to generate code for {}: {}",
                template_file.display(),
                e
            )
        });

        let output_file = generated_dir.join(format!(
            "{}.rs",
            template_file
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_lowercase()
        ));

        fs::write(&output_file, generated_code.to_string())
            .expect("Failed to write generated file");

        // Verify the generated file exists and contains expected content
        let generated_content =
            fs::read_to_string(&output_file).expect("Failed to read generated file");
        assert!(generated_content.contains("Component"));
        assert!(generated_content.contains("Props"));
    }
}

#[test]
fn test_prop_validation() {
    let template = r#"
component ValidatedForm {
    props {
        email: String,
        age: u32,
        name: String?,
        terms_accepted: bool = false,
    }
}

ruitl ValidatedForm(props: ValidatedFormProps) {
    <form class="validated-form">
        <input type="email" value={props.email} required />
        <input type="number" value={props.age.to_string()} min="0" max="120" />
        if let Some(name) = props.name {
            <input type="text" value={name} placeholder="Name" />
        }
        <input type="checkbox" checked?={props.terms_accepted} />
    </form>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser.parse().expect("Failed to parse validation template");

    let component = &ast.components[0];
    assert_eq!(component.props.len(), 4);

    // Check prop types and optionality
    let email_prop = component.props.iter().find(|p| p.name == "email").unwrap();
    assert_eq!(email_prop.prop_type, "String");
    assert!(!email_prop.optional);

    let name_prop = component.props.iter().find(|p| p.name == "name").unwrap();
    assert_eq!(name_prop.prop_type, "String?");
    assert!(!name_prop.optional); // ? in type, not optional flag

    let terms_prop = component
        .props
        .iter()
        .find(|p| p.name == "terms_accepted")
        .unwrap();
    assert_eq!(terms_prop.prop_type, "bool");
    assert!(terms_prop.optional);
    assert_eq!(terms_prop.default_value, Some("false".to_string()));

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("impl ruitl::component::ComponentProps"));
    assert!(code_str.contains("fn validate"));
}

#[test]
fn test_fragment_rendering() {
    let template = r#"
component Fragment {
    props {
        items: Vec<String>,
    }
}

ruitl Fragment(props: FragmentProps) {
    for item in props.items {
        <span class="item">{item}</span>
        <span class="separator"> | </span>
    }
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser.parse().expect("Failed to parse fragment template");

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("Html::fragment"));
}

#[test]
fn test_raw_html_handling() {
    let template = r#"
component RawContent {
    props {
        html_content: String,
        safe_content: String,
    }
}

ruitl RawContent(props: RawContentProps) {
    <div class="content">
        <div class="safe">{props.safe_content}</div>
        <div class="raw" dangerously_set_inner_html={props.html_content}></div>
    </div>
}
"#;

    let mut parser = RuitlParser::new(template.to_string());
    let ast = parser.parse().expect("Failed to parse raw HTML template");

    let mut generator = CodeGenerator::new(ast);
    let generated_code = generator.generate().expect("Failed to generate code");

    let code_str = generated_code.to_string();
    assert!(code_str.contains("props.safe_content"));
    assert!(code_str.contains("props.html_content"));
}
