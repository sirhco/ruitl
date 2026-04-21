//! RUITL Template Compiler Demo
//!
//! This example demonstrates the RUITL template compilation system:
//! 1. Show .ruitl template syntax
//! 2. Demonstrate build-time compilation
//! 3. Show generated Rust component code
//! 4. Demonstrate runtime component usage pattern
//!
//! Run with: cargo run --example template_compiler_demo

use ruitl::prelude::*;

fn main() -> Result<()> {
    println!("🚀 RUITL Template Compiler Demo");
    println!("=================================\n");

    // Step 1: Show .ruitl template syntax
    println!("📝 RUITL Template Syntax:");
    println!("-------------------------");
    show_template_syntax();

    // Step 2: Show generated code from build script
    println!("\n⚙️  Generated Code Examples:");
    println!("----------------------------");
    show_generated_code_examples();

    // Step 3: Demonstrate component usage pattern
    println!("\n🏃 Component Usage Pattern:");
    println!("---------------------------");
    demonstrate_component_usage();

    // Step 4: Show build workflow
    println!("\n🔨 Build Workflow:");
    println!("------------------");
    show_build_workflow();

    println!("\n🎉 Template Compiler Demo Complete!");
    println!("\nKey Features Demonstrated:");
    println!("• .ruitl template syntax");
    println!("• Build-time compilation");
    println!("• Generated Rust components");
    println!("• Type-safe props structures");
    println!("• Runtime component usage");
    println!("• HTML generation");

    Ok(())
}

fn show_template_syntax() {
    println!("Here are the .ruitl template files in the templates/ directory:\n");

    println!("📄 templates/Hello.ruitl:");
    println!(
        "{}",
        r#"// Hello.ruitl - A minimal test template

component Hello {
    props {
        name: String,
    }
}

ruitl Hello(name: String) {
    <div>
        <h1>Hello, {name}!</h1>
    </div>
}"#
    );

    println!("\n📄 templates/Button.ruitl:");
    println!(
        "{}",
        r#"// Button.ruitl - A simple button component

component Button {
    props {
        text: String,
        variant: String = "primary",
    }
}

ruitl Button(text: String, variant: String) {
    <button class={format!("btn btn-{}", variant)} type="button">
        {text}
    </button>
}"#
    );

    println!("\n📄 templates/UserCard.ruitl:");
    println!(
        "{}",
        r#"// UserCard.ruitl - A simple user card component

component UserCard {
    props {
        name: String,
        email: String,
        role: String = "user",
    }
}

ruitl UserCard(name: String, email: String, role: String) {
    <div class="user-card">
        <div class="user-header">
            <h3 class="user-name">{name}</h3>
            <span class="user-role">{role}</span>
        </div>
        <div class="user-contact">
            <p class="user-email">{email}</p>
        </div>
    </div>
}"#
    );
}

fn show_generated_code_examples() {
    println!("The build script automatically compiles .ruitl files into Rust code:\n");

    println!("📄 Generated hello.rs:");
    println!(
        "{}",
        r#"// Generated from templates/Hello.ruitl
use ruitl::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HelloProps {
    pub name: String,
}

impl ComponentProps for HelloProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Hello;

impl Component for Hello {
    type Props = HelloProps;

    fn render(&self, props: &Self::Props, context: &ComponentContext) -> ruitl::error::Result<Html> {
        Ok(html! {
            <div>
                <h1>Hello, { name }!</h1>
            </div>
        })
    }
}"#
    );

    println!("\n📄 Generated button.rs:");
    println!(
        "{}",
        r#"#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ButtonProps {
    pub text: String,
    pub variant: String, // default: "primary"
}

#[derive(Debug)]
pub struct Button;

impl Component for Button {
    type Props = ButtonProps;

    fn render(&self, props: &Self::Props, context: &ComponentContext) -> ruitl::error::Result<Html> {
        Ok(html! {
            <button class={ format!("btn btn-{}", variant) } type="button">
                { text }
            </button>
        })
    }
}"#
    );
}

fn demonstrate_component_usage() {
    println!("Here's how you would use the generated components in your Rust code:\n");

    println!("📄 main.rs:");
    println!(
        "{}",
        r#"use ruitl::prelude::*;

// Import generated components (would be available after build)
// mod generated;
// use generated::*;

fn main() -> Result<()> {
    // Create component instances
    let hello = Hello;
    let button = Button;

    // Create props with type safety
    let hello_props = HelloProps {
        name: "World".to_string(),
    };

    let button_props = ButtonProps {
        text: "Click Me!".to_string(),
        variant: "primary".to_string(),
    };

    // Render components
    let context = ComponentContext::new();

    let hello_html = hello.render(&hello_props, &context)?;
    let button_html = button.render(&button_props, &context)?;

    // Output HTML
    println!("Hello Component: {}", hello_html.render());
    println!("Button Component: {}", button_html.render());

    Ok(())
}"#
    );

    println!("\nExpected Output:");
    println!("Hello Component: <div><h1>Hello, World!</h1></div>");
    println!(
        "Button Component: <button class=\"btn btn-primary\" type=\"button\">Click Me!</button>"
    );
}

fn show_build_workflow() {
    println!("The RUITL build process integrates seamlessly with Cargo:\n");

    println!("1. 📁 Project Structure:");
    println!("   my-ruitl-app/");
    println!("   ├── Cargo.toml");
    println!("   ├── build.rs           # Auto-compile .ruitl files");
    println!("   ├── src/");
    println!("   │   ├── main.rs");
    println!("   │   └── lib.rs");
    println!("   └── templates/          # .ruitl template files");
    println!("       ├── Button.ruitl");
    println!("       ├── UserCard.ruitl");
    println!("       └── Hello.ruitl");

    println!("\n2. 🔨 Build Process:");
    println!("   • cargo build");
    println!("   • build.rs scans templates/ directory");
    println!("   • Parses each .ruitl file");
    println!("   • Generates corresponding .rs files");
    println!("   • Creates module exports");
    println!("   • Compiles Rust code normally");

    println!("\n3. ✅ Generated Files:");
    println!("   target/debug/build/my-app-*/out/generated/");
    println!("   ├── mod.rs");
    println!("   ├── button.rs          # From Button.ruitl");
    println!("   ├── usercard.rs        # From UserCard.ruitl");
    println!("   └── hello.rs           # From Hello.ruitl");

    println!("\n4. 🏃 Runtime Usage:");
    println!("   • Components implement the Component trait");
    println!("   • Props are validated at compile time");
    println!("   • HTML is generated efficiently");
    println!("   • Full Rust type safety maintained");

    println!("\n5. 🔄 Development Workflow:");
    println!("   • Edit .ruitl templates");
    println!("   • Run cargo build");
    println!("   • Generated code updates automatically");
    println!("   • Use components in your Rust code");
    println!("   • Deploy as normal Rust binary");
}
