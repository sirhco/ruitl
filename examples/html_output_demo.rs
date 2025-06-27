//! HTML Output Demo for RUITL
//!
//! This example demonstrates how RUITL components generate HTML that can be
//! directly opened in web browsers. It creates several HTML files showing
//! different component outputs.
//!
//! Run with: cargo run --example html_output_demo
//! Then open the generated HTML files in your browser.

use ruitl::prelude::*;
use std::fs;
use std::io::Write;

// Example components (in real usage, these come from .ruitl files)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageProps {
    pub title: String,
    pub description: String,
}

impl ComponentProps for PageProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        if self.title.is_empty() {
            return Err(RuitlError::validation("Title cannot be empty"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Page;

impl Component for Page {
    type Props = PageProps;

    fn render(
        &self,
        props: &Self::Props,
        _context: &ComponentContext,
    ) -> ruitl::error::Result<Html> {
        use ruitl::html::*;
        Ok(Html::Element(
            html()
                .child(Html::Element(
                    head()
                        .child(Html::Element(title().text(&props.title)))
                        .child(Html::Element(style().text(
                            "
                            body {
                                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                                line-height: 1.6;
                                margin: 0;
                                padding: 20px;
                                background: #f5f5f5;
                            }
                            .container {
                                max-width: 800px;
                                margin: 0 auto;
                                background: white;
                                padding: 40px;
                                border-radius: 8px;
                                box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                            }
                            .button {
                                display: inline-block;
                                background: #007bff;
                                color: white;
                                padding: 12px 24px;
                                border: none;
                                border-radius: 6px;
                                cursor: pointer;
                                text-decoration: none;
                                font-weight: 500;
                                margin: 8px 8px 8px 0;
                                transition: background 0.2s;
                            }
                            .button:hover { background: #0056b3; }
                            .button.secondary { background: #6c757d; }
                            .button.secondary:hover { background: #545b62; }
                            .button.success { background: #28a745; }
                            .button.success:hover { background: #1e7e34; }
                            .card {
                                border: 1px solid #e9ecef;
                                padding: 24px;
                                margin: 20px 0;
                                border-radius: 8px;
                                background: #f8f9fa;
                            }
                            .user-grid {
                                display: grid;
                                grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
                                gap: 20px;
                                margin: 20px 0;
                            }
                            .conditional-demo {
                                border: 2px dashed #007bff;
                                padding: 20px;
                                margin: 20px 0;
                                border-radius: 8px;
                            }
                            h1 { color: #343a40; margin-bottom: 10px; }
                            h2 { color: #495057; border-bottom: 2px solid #e9ecef; padding-bottom: 10px; }
                            .meta { color: #6c757d; font-style: italic; margin-bottom: 30px; }
                            code {
                                background: #f8f9fa;
                                padding: 2px 6px;
                                border-radius: 3px;
                                font-family: 'Monaco', 'Consolas', monospace;
                                color: #e83e8c;
                            }
                            "
                        ))),
                ))
                .child(Html::Element(
                    body().child(Html::Element(
                        div()
                            .class("container")
                            .child(Html::Element(h1().text(&props.title)))
                            .child(Html::Element(p().class("meta").text(&props.description))),
                    )),
                )),
        ))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ButtonProps {
    pub text: String,
    pub variant: String,
    pub href: Option<String>,
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

        let class_name = format!("button {}", props.variant);

        if let Some(href) = &props.href {
            Ok(Html::Element(
                a().attr("href", href)
                    .attr("class", &class_name)
                    .text(&props.text),
            ))
        } else {
            Ok(Html::Element(
                button()
                    .attr("class", &class_name)
                    .attr("type", "button")
                    .text(&props.text),
            ))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserCardProps {
    pub name: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
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

        let status_text = if props.is_active {
            "Active"
        } else {
            "Inactive"
        };
        let status_color = if props.is_active {
            "#28a745"
        } else {
            "#6c757d"
        };

        Ok(Html::Element(
            div()
                .class("card")
                .child(Html::Element(h3().text(&format!("👤 {}", props.name))))
                .child(Html::Element(p().text(&format!("📧 {}", props.email))))
                .child(Html::Element(p().text(&format!("🔖 Role: {}", props.role))))
                .child(Html::Element(
                    p().child(Html::Element(
                        span()
                            .attr(
                                "style",
                                &format!("color: {}; font-weight: bold;", status_color),
                            )
                            .text(&format!("● Status: {}", status_text)),
                    )),
                )),
        ))
    }
}

fn generate_basic_demo() -> ruitl::error::Result<String> {
    let page = Page;
    let props = PageProps {
        title: "RUITL Basic Components Demo".to_string(),
        description: "This page demonstrates basic RUITL components rendered as HTML".to_string(),
    };

    let context = ComponentContext::new();
    let mut page_html = page.render(&props, &context)?.render();

    // Add buttons section
    let button_demos = vec![
        ("Primary Button", "primary", None),
        ("Secondary Button", "secondary", None),
        (
            "Success Link",
            "success",
            Some("https://github.com/ruitl/ruitl"),
        ),
    ];

    let mut buttons_html = String::from("<h2>Button Components</h2><p>These buttons are generated from RUITL components with different props:</p>");

    for (text, variant, href) in button_demos {
        let button = Button;
        let button_props = ButtonProps {
            text: text.to_string(),
            variant: variant.to_string(),
            href: href.map(|h| h.to_string()),
        };

        let button_html = button.render(&button_props, &context)?.render();
        buttons_html.push_str(&button_html);
    }

    // Insert content before closing body tag
    page_html = page_html.replace("</div></body>", &format!("{}</div></body>", buttons_html));

    Ok(page_html)
}

fn generate_conditional_demo() -> ruitl::error::Result<String> {
    let page = Page;
    let props = PageProps {
        title: "RUITL Conditional Rendering Demo".to_string(),
        description: "This page demonstrates conditional rendering with boolean props".to_string(),
    };

    let context = ComponentContext::new();
    let mut page_html = page.render(&props, &context)?.render();

    // Demonstrate conditional rendering
    let users = vec![
        ("Alice Johnson", "alice@company.com", "Admin", true),
        ("Bob Smith", "bob@company.com", "User", false),
        ("Carol Davis", "carol@company.com", "Moderator", true),
    ];

    let mut conditional_html = String::from(
        "<h2>Conditional Rendering Demo</h2>
         <p>These user cards show conditional status rendering based on <code>is_active</code> boolean prop:</p>
         <div class=\"user-grid\">"
    );

    for (name, email, role, is_active) in users {
        let user_card = UserCard;
        let user_props = UserCardProps {
            name: name.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            is_active,
        };

        let user_html = user_card.render(&user_props, &context)?.render();
        conditional_html.push_str(&user_html);
    }

    conditional_html.push_str("</div>");

    // Add explanation
    conditional_html.push_str(
        "<div class=\"conditional-demo\">
         <h3>🔧 How It Works</h3>
         <p>Each user card uses conditional rendering in the RUITL component:</p>
         <ul>
             <li><strong>Active users</strong> show green status with <code>is_active: true</code></li>
             <li><strong>Inactive users</strong> show gray status with <code>is_active: false</code></li>
             <li>The component uses Rust's <code>if</code> expressions to choose colors and text</li>
             <li>All logic is executed at render time, producing clean HTML</li>
         </ul>
         </div>"
    );

    // Insert content before closing body tag
    page_html = page_html.replace(
        "</div></body>",
        &format!("{}</div></body>", conditional_html),
    );

    Ok(page_html)
}

fn generate_composition_demo() -> ruitl::error::Result<String> {
    let page = Page;
    let props = PageProps {
        title: "RUITL Component Composition Demo".to_string(),
        description: "This page demonstrates composing multiple RUITL components together"
            .to_string(),
    };

    let context = ComponentContext::new();
    let mut page_html = page.render(&props, &context)?.render();

    // Create a complex composed layout
    let mut composition_html = String::from("<h2>Component Composition</h2>");

    // Header section with buttons
    composition_html.push_str("<h3>Navigation Buttons</h3>");
    let nav_buttons = vec![
        ("🏠 Home", "primary", Some("#home")),
        ("👥 Users", "secondary", Some("#users")),
        ("⚙️ Settings", "secondary", Some("#settings")),
    ];

    for (text, variant, href) in nav_buttons {
        let button = Button;
        let button_props = ButtonProps {
            text: text.to_string(),
            variant: variant.to_string(),
            href: href.map(|h| h.to_string()),
        };
        composition_html.push_str(&button.render(&button_props, &context)?.render());
    }

    // User showcase section
    composition_html.push_str("<h3 id=\"users\">Team Members</h3><div class=\"user-grid\">");

    let team_members = vec![
        (
            "Dr. Sarah Chen",
            "sarah.chen@ruitl.dev",
            "Lead Engineer",
            true,
        ),
        (
            "Marcus Rodriguez",
            "marcus.r@ruitl.dev",
            "Frontend Developer",
            true,
        ),
        ("Emma Thompson", "emma.t@ruitl.dev", "UI/UX Designer", false),
        ("James Wilson", "james.w@ruitl.dev", "DevOps Engineer", true),
    ];

    for (name, email, role, is_active) in team_members {
        let user_card = UserCard;
        let user_props = UserCardProps {
            name: name.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            is_active,
        };
        composition_html.push_str(&user_card.render(&user_props, &context)?.render());
    }

    composition_html.push_str("</div>");

    // Action buttons section
    composition_html.push_str("<h3 id=\"settings\">Actions</h3>");
    let action_buttons = vec![
        ("✉️ Send Invites", "success"),
        ("📊 Generate Report", "primary"),
        ("🗑️ Archive Inactive", "secondary"),
    ];

    for (text, variant) in action_buttons {
        let button = Button;
        let button_props = ButtonProps {
            text: text.to_string(),
            variant: variant.to_string(),
            href: None,
        };
        composition_html.push_str(&button.render(&button_props, &context)?.render());
    }

    // Add technical explanation
    composition_html.push_str(
        "<div class=\"conditional-demo\">
         <h3>🎯 Component Composition Benefits</h3>
         <ul>
             <li><strong>Reusability:</strong> Same Button and UserCard components used multiple times</li>
             <li><strong>Type Safety:</strong> Each component validates its props at compile time</li>
             <li><strong>Maintainability:</strong> Changes to components automatically update all usages</li>
             <li><strong>Performance:</strong> Zero runtime overhead - all HTML pre-generated</li>
             <li><strong>Consistency:</strong> Shared styling and behavior across all instances</li>
         </ul>
         </div>"
    );

    // Insert content before closing body tag
    page_html = page_html.replace(
        "</div></body>",
        &format!("{}</div></body>", composition_html),
    );

    Ok(page_html)
}

fn save_html_file(filename: &str, content: &str) -> std::io::Result<()> {
    fs::create_dir_all("output")?;
    let path = format!("output/{}", filename);
    let mut file = fs::File::create(&path)?;
    file.write_all(content.as_bytes())?;
    println!("✅ Generated: {}", path);
    Ok(())
}

fn main() -> ruitl::error::Result<()> {
    println!("🎨 RUITL HTML Output Demo");
    println!("========================");
    println!();

    // Generate different demo pages
    println!("🔧 Generating HTML demos...");

    let basic_demo = generate_basic_demo()?;
    save_html_file("basic_demo.html", &basic_demo)?;

    let conditional_demo = generate_conditional_demo()?;
    save_html_file("conditional_demo.html", &conditional_demo)?;

    let composition_demo = generate_composition_demo()?;
    save_html_file("composition_demo.html", &composition_demo)?;

    // Generate an index page
    let index_html = format!(
        "<!DOCTYPE html>
        <html>
        <head>
            <title>RUITL HTML Output Demos</title>
            <style>
                body {{
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                    line-height: 1.6; margin: 0; padding: 40px; background: #f5f5f5;
                }}
                .container {{
                    max-width: 600px; margin: 0 auto; background: white;
                    padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                }}
                .demo-link {{
                    display: block; background: #007bff; color: white; padding: 15px 20px;
                    text-decoration: none; margin: 10px 0; border-radius: 6px;
                    transition: background 0.2s;
                }}
                .demo-link:hover {{ background: #0056b3; }}
                h1 {{ color: #343a40; }}
                p {{ color: #6c757d; }}
            </style>
        </head>
        <body>
            <div class=\"container\">
                <h1>🎨 RUITL HTML Output Demos</h1>
                <p>Choose a demo to see how RUITL components render HTML for browsers:</p>

                <a href=\"basic_demo.html\" class=\"demo-link\">
                    📦 Basic Components Demo<br>
                    <small style=\"opacity: 0.8;\">Simple buttons and page layout</small>
                </a>

                <a href=\"conditional_demo.html\" class=\"demo-link\">
                    🔀 Conditional Rendering Demo<br>
                    <small style=\"opacity: 0.8;\">Boolean props and dynamic content</small>
                </a>

                <a href=\"composition_demo.html\" class=\"demo-link\">
                    🧩 Component Composition Demo<br>
                    <small style=\"opacity: 0.8;\">Multiple components working together</small>
                </a>

                <hr style=\"margin: 30px 0; border: none; border-top: 1px solid #e9ecef;\">

                <h2>🚀 How It Works</h2>
                <ol>
                    <li><strong>RUITL Components:</strong> Written in Rust with type-safe props</li>
                    <li><strong>HTML Generation:</strong> Components render to HTML strings</li>
                    <li><strong>Browser Ready:</strong> Standard HTML/CSS that works everywhere</li>
                    <li><strong>No JavaScript:</strong> Pure server-side rendering</li>
                </ol>

                <p style=\"background: #f8f9fa; padding: 15px; border-radius: 6px; margin-top: 20px;\">
                    <strong>💡 Try it:</strong> View source on any of these pages to see the clean,
                    semantic HTML generated by RUITL components!
                </p>
            </div>
        </body>
        </html>"
    );

    save_html_file("index.html", &index_html)?;

    println!();
    println!("🌐 Open these files in your browser:");
    println!("   📂 output/index.html           - Main demo index");
    println!("   📄 output/basic_demo.html      - Basic components");
    println!("   📄 output/conditional_demo.html - Conditional rendering");
    println!("   📄 output/composition_demo.html - Component composition");
    println!();
    println!("✨ These HTML files demonstrate:");
    println!("   ✅ Type-safe component props");
    println!("   ✅ Server-side HTML generation");
    println!("   ✅ Conditional rendering with boolean props");
    println!("   ✅ Component reusability and composition");
    println!("   ✅ Clean, semantic HTML output");
    println!("   ✅ CSS styling integration");
    println!();
    println!("🔧 Generated with RUITL - no JavaScript needed!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_component() {
        let page = Page;
        let props = PageProps {
            title: "Test Page".to_string(),
            description: "Test description".to_string(),
        };
        let context = ComponentContext::new();

        let result = page.render(&props, &context);
        assert!(result.is_ok());

        let html = result.unwrap().render();
        assert!(html.contains("Test Page"));
        assert!(html.contains("Test description"));
        assert!(html.contains("<!DOCTYPE html") || html.contains("<html"));
    }

    #[test]
    fn test_button_variants() {
        let button = Button;
        let context = ComponentContext::new();

        // Test button element
        let props = ButtonProps {
            text: "Click me".to_string(),
            variant: "primary".to_string(),
            href: None,
        };
        let html = button.render(&props, &context).unwrap().render();
        assert!(html.contains("<button"));
        assert!(html.contains("button primary"));

        // Test link element
        let props = ButtonProps {
            text: "Link".to_string(),
            variant: "secondary".to_string(),
            href: Some("https://example.com".to_string()),
        };
        let html = button.render(&props, &context).unwrap().render();
        assert!(html.contains("<a href"));
        assert!(html.contains("button secondary"));
    }

    #[test]
    fn test_user_card_conditional() {
        let user_card = UserCard;
        let context = ComponentContext::new();

        // Test active user
        let props = UserCardProps {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "Admin".to_string(),
            is_active: true,
        };
        let html = user_card.render(&props, &context).unwrap().render();
        assert!(html.contains("Active"));
        assert!(html.contains("#28a745")); // Green color

        // Test inactive user
        let props = UserCardProps {
            name: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
            role: "User".to_string(),
            is_active: false,
        };
        let html = user_card.render(&props, &context).unwrap().render();
        assert!(html.contains("Inactive"));
        assert!(html.contains("#6c757d")); // Gray color
    }

    #[test]
    fn test_demo_generation() {
        let basic_demo = generate_basic_demo();
        assert!(basic_demo.is_ok());

        let conditional_demo = generate_conditional_demo();
        assert!(conditional_demo.is_ok());

        let composition_demo = generate_composition_demo();
        assert!(composition_demo.is_ok());
    }
}
