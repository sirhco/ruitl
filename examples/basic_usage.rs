//! Basic Usage Example for RUITL Template Compiler
//!
//! This example demonstrates how to use components generated from .ruitl templates.
//!
//! Prerequisites:
//! 1. Have .ruitl template files in templates/ directory
//! 2. Run `cargo build` to compile templates
//! 3. Generated components will be available for use
//!
//! Run with: cargo run --example basic_usage

use ruitl::prelude::*;

// In a real project, these would be generated automatically from .ruitl files
// For demonstration, we'll show what the generated code looks like

// Generated from templates/Hello.ruitl
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HelloProps {
    pub name: String,
}

impl ComponentProps for HelloProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        if self.name.is_empty() {
            return Err(RuitlError::validation("Name cannot be empty"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Hello;

impl Component for Hello {
    type Props = HelloProps;

    fn render(
        &self,
        props: &Self::Props,
        _context: &ComponentContext,
    ) -> ruitl::error::Result<Html> {
        use ruitl::html::*;
        Ok(Html::Element(div().class("greeting").child(Html::Element(
            h1().text(&format!("Hello, {}!", props.name)),
        ))))
    }
}

// Generated from templates/Button.ruitl
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ButtonProps {
    pub text: String,
    pub variant: String, // default: "primary"
    pub disabled: bool,  // default: false
}

impl ComponentProps for ButtonProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        let valid_variants = vec!["primary", "secondary", "success", "danger"];
        if !valid_variants.contains(&self.variant.as_str()) {
            return Err(RuitlError::validation(&format!(
                "Invalid variant '{}'. Must be one of: {:?}",
                self.variant, valid_variants
            )));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Button;

impl Component for Button {
    type Props = ButtonProps;

    fn render(
        &self,
        props: &Self::Props,
        _context: &ComponentContext,
    ) -> ruitl::error::Result<Html> {
        use ruitl::html::*;
        let classes = format!("btn btn-{}", props.variant);

        let mut button_elem = button()
            .class(&classes)
            .attr("type", "button")
            .text(&props.text);

        if props.disabled {
            button_elem = button_elem.attr("disabled", "disabled");
        }

        Ok(Html::Element(button_elem))
    }
}

// Generated from templates/UserCard.ruitl
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserCardProps {
    pub name: String,
    pub email: String,
    pub role: String, // default: "user"
    pub avatar_url: Option<String>,
    pub is_online: bool, // default: false
}

impl ComponentProps for UserCardProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        if self.name.is_empty() {
            return Err(RuitlError::validation("Name is required"));
        }
        if !self.email.contains('@') {
            return Err(RuitlError::validation("Invalid email format"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct UserCard;

impl Component for UserCard {
    type Props = UserCardProps;

    fn render(
        &self,
        props: &Self::Props,
        _context: &ComponentContext,
    ) -> ruitl::error::Result<Html> {
        use ruitl::html::*;
        let status_class = format!(
            "status {}",
            if props.is_online { "online" } else { "offline" }
        );
        let status_symbol = if props.is_online { "●" } else { "○" };
        let initials = props
            .name
            .chars()
            .take(2)
            .collect::<String>()
            .to_uppercase();

        let avatar_elem = if let Some(ref url) = props.avatar_url {
            img()
                .attr("src", url)
                .attr("alt", &format!("{}'s avatar", props.name))
                .class("avatar")
        } else {
            div().class("avatar-placeholder").text(&initials)
        };

        Ok(Html::Element(
            div()
                .class("user-card")
                .child(Html::Element(
                    div()
                        .class("user-header")
                        .child(Html::Element(avatar_elem))
                        .child(Html::Element(
                            div()
                                .class("user-info")
                                .child(Html::Element(h3().class("user-name").text(&props.name)))
                                .child(Html::Element(span().class("user-role").text(&props.role)))
                                .child(Html::Element(
                                    span().class(&status_class).text(status_symbol),
                                )),
                        )),
                ))
                .child(Html::Element(div().class("user-contact").child(
                    Html::Element(p().class("user-email").text(&props.email)),
                ))),
        ))
    }
}

fn main() -> Result<()> {
    println!("🚀 RUITL Basic Usage Example");
    println!("============================\n");

    // Create a component context
    let context = ComponentContext::new();

    // Example 1: Simple Hello component
    println!("📝 Example 1: Hello Component");
    println!("------------------------------");

    let hello = Hello;
    let hello_props = HelloProps {
        name: "RUITL".to_string(),
    };

    // Validate props (optional - happens automatically during render)
    hello_props.validate()?;

    let hello_html = hello.render(&hello_props, &context)?;
    println!("Props: {:?}", hello_props);
    println!("HTML: {}", hello_html.render());
    println!();

    // Example 2: Button component with variants
    println!("📝 Example 2: Button Components");
    println!("-------------------------------");

    let button = Button;

    let button_variants = vec![
        ("primary", false),
        ("secondary", false),
        ("danger", false),
        ("success", true), // disabled
    ];

    for (variant, disabled) in button_variants {
        let props = ButtonProps {
            text: format!("{} Button", variant.to_uppercase()),
            variant: variant.to_string(),
            disabled,
        };

        let html = button.render(&props, &context)?;
        println!("Variant: {} | HTML: {}", variant, html.render());
    }
    println!();

    // Example 3: UserCard component with different data
    println!("📝 Example 3: UserCard Components");
    println!("---------------------------------");

    let user_card = UserCard;

    let users = vec![
        UserCardProps {
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            role: "admin".to_string(),
            avatar_url: Some("https://example.com/alice.jpg".to_string()),
            is_online: true,
        },
        UserCardProps {
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            role: "developer".to_string(),
            avatar_url: None,
            is_online: false,
        },
        UserCardProps {
            name: "Carol Davis".to_string(),
            email: "carol@example.com".to_string(),
            role: "designer".to_string(),
            avatar_url: Some("https://example.com/carol.jpg".to_string()),
            is_online: true,
        },
    ];

    for (i, user_props) in users.iter().enumerate() {
        println!("User {}: {}", i + 1, user_props.name);
        let html = user_card.render(user_props, &context)?;
        println!("HTML: {}", html.render());
        println!();
    }

    // Example 4: Error handling
    println!("📝 Example 4: Error Handling");
    println!("----------------------------");

    // Try to create invalid props
    let invalid_button_props = ButtonProps {
        text: "Invalid Button".to_string(),
        variant: "invalid_variant".to_string(),
        disabled: false,
    };

    match invalid_button_props.validate() {
        Ok(_) => println!("Props are valid"),
        Err(e) => println!("Validation error: {}", e),
    }

    let invalid_user_props = UserCardProps {
        name: "".to_string(),               // Empty name
        email: "invalid-email".to_string(), // No @ symbol
        role: "user".to_string(),
        avatar_url: None,
        is_online: false,
    };

    match invalid_user_props.validate() {
        Ok(_) => println!("Props are valid"),
        Err(e) => println!("Validation error: {}", e),
    }
    println!();

    // Example 5: Combining components (composition)
    println!("📝 Example 5: Component Composition");
    println!("-----------------------------------");

    // Create a simple page that uses multiple components
    let page_html = create_sample_page(&context)?;
    println!("Complete page HTML:");
    println!("{}", page_html.render());

    println!("\n✅ All examples completed successfully!");
    println!("\n💡 Tips:");
    println!("  • Templates are compiled at build time for zero runtime overhead");
    println!("  • Props are validated automatically during rendering");
    println!("  • Components implement the standard Component trait");
    println!("  • HTML output is properly escaped for security");
    println!("  • Error handling is built into the component system");

    Ok(())
}

fn create_sample_page(context: &ComponentContext) -> Result<Html> {
    // This demonstrates how you might compose multiple components
    // into a larger page structure

    let hello = Hello;
    let button = Button;
    let user_card = UserCard;

    // Render individual components
    let header_html = hello.render(
        &HelloProps {
            name: "Sample Page".to_string(),
        },
        context,
    )?;

    let cta_button_html = button.render(
        &ButtonProps {
            text: "Get Started".to_string(),
            variant: "primary".to_string(),
            disabled: false,
        },
        context,
    )?;

    let user_html = user_card.render(
        &UserCardProps {
            name: "Demo User".to_string(),
            email: "demo@example.com".to_string(),
            role: "visitor".to_string(),
            avatar_url: None,
            is_online: true,
        },
        context,
    )?;

    // Compose into a complete page using the HTML builder API
    use ruitl::html::*;

    let styles = r#"
        body { font-family: Arial, sans-serif; margin: 0; padding: 20px; }
        .greeting { text-align: center; margin-bottom: 30px; }
        .btn { padding: 10px 20px; margin: 10px; border: none; border-radius: 4px; cursor: pointer; }
        .btn-primary { background: #007bff; color: white; }
        .btn-secondary { background: #6c757d; color: white; }
        .btn-success { background: #28a745; color: white; }
        .btn-danger { background: #dc3545; color: white; }
        .user-card { border: 1px solid #ddd; padding: 15px; border-radius: 8px; margin: 20px 0; }
        .user-header { display: flex; align-items: center; gap: 10px; }
        .avatar { width: 40px; height: 40px; border-radius: 50%; }
        .avatar-placeholder { width: 40px; height: 40px; border-radius: 50%; background: #ccc; display: flex; align-items: center; justify-content: center; font-weight: bold; }
        .status.online { color: green; }
        .status.offline { color: gray; }
    "#;

    Ok(Html::Element(
        html()
            .child(Html::Element(
                head()
                    .child(Html::Element(title().text("RUITL Demo Page")))
                    .child(Html::Element(style().text(styles))),
            ))
            .child(Html::Element(
                body().child(Html::Element(
                    main()
                        .child(header_html)
                        .child(Html::Element(
                            section()
                                .child(Html::Element(h2().text("Actions")))
                                .child(cta_button_html),
                        ))
                        .child(Html::Element(
                            section()
                                .child(Html::Element(h2().text("User Profile")))
                                .child(user_html),
                        ))
                        .child(Html::Element(footer().child(Html::Element(
                            p().text("Generated with RUITL Template Compiler"),
                        )))),
                )),
            )),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_component() {
        let hello = Hello;
        let props = HelloProps {
            name: "Test".to_string(),
        };
        let context = ComponentContext::new();

        let result = hello.render(&props, &context);
        assert!(result.is_ok());

        let html = result.unwrap();
        let rendered = html.render();
        assert!(rendered.contains("Hello, Test!"));
    }

    #[test]
    fn test_button_validation() {
        let valid_props = ButtonProps {
            text: "Click me".to_string(),
            variant: "primary".to_string(),
            disabled: false,
        };
        assert!(valid_props.validate().is_ok());

        let invalid_props = ButtonProps {
            text: "Click me".to_string(),
            variant: "invalid".to_string(),
            disabled: false,
        };
        assert!(invalid_props.validate().is_err());
    }

    #[test]
    fn test_user_card_validation() {
        let valid_props = UserCardProps {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "user".to_string(),
            avatar_url: None,
            is_online: false,
        };
        assert!(valid_props.validate().is_ok());

        let invalid_props = UserCardProps {
            name: "".to_string(), // Empty name
            email: "invalid-email".to_string(),
            role: "user".to_string(),
            avatar_url: None,
            is_online: false,
        };
        assert!(invalid_props.validate().is_err());
    }
}
