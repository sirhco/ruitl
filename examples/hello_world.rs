//! Hello World example for RUITL
//!
//! This example demonstrates the basic capabilities of RUITL including:
//! - Creating components with props
//! - HTML generation
//! - Basic component rendering

use ruitl::prelude::*;

// Define props for our greeting component
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GreetingProps {
    pub name: String,
    pub message: Option<String>,
}

impl ComponentProps for GreetingProps {}

// Greeting component
#[derive(Debug)]
pub struct Greeting;

impl Component for Greeting {
    type Props = GreetingProps;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let message = props.message.as_deref().unwrap_or("Hello");

        Ok(Html::Element(
            ruitl::html::div()
                .child(Html::Element(
                    ruitl::html::h1().text(&format!("{}, {}!", message, props.name)),
                ))
                .child(Html::Element(ruitl::html::p().text("Welcome to RUITL!"))),
        ))
    }
}

// Simple page layout component
#[derive(Debug)]
pub struct Layout;

impl Component for Layout {
    type Props = EmptyProps;

    fn render(&self, _props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        Ok(Html::Element(
            ruitl::html::html()
                .child(Html::Element(ruitl::html::head().child(Html::Element(
                    ruitl::html::HtmlElement::new("title").text("RUITL Hello World"),
                ))))
                .child(Html::Element(
                    ruitl::html::body().child(Html::Element(
                        ruitl::html::div()
                            .child(Html::Element(ruitl::html::h1().text("RUITL Demo")))
                            .child(Html::Element(
                                ruitl::html::p().text("This is a basic RUITL example."),
                            )),
                    )),
                )),
        ))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 RUITL Hello World Example");

    // Create greeting component with props
    let greeting = Greeting;
    let greeting_props = GreetingProps {
        name: "World".to_string(),
        message: Some("Hello".to_string()),
    };

    // Render the greeting component
    let context = ComponentContext::new();
    let greeting_html = greeting.render(&greeting_props, &context)?;
    println!("Greeting HTML: {}", greeting_html.render());

    // Create and render layout component
    let layout = Layout;
    let layout_html = layout.render(&EmptyProps, &context)?;
    println!("Layout HTML: {}", layout_html.render());

    // Demonstrate different greeting messages
    let greetings = vec![("Alice", "Hi"), ("Bob", "Hey"), ("Charlie", "Howdy")];

    for (name, message) in greetings {
        let props = GreetingProps {
            name: name.to_string(),
            message: Some(message.to_string()),
        };
        let html = greeting.render(&props, &context)?;
        println!("{}: {}", name, html.render());
    }

    println!("✅ Example completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_component() {
        let greeting = Greeting;
        let props = GreetingProps {
            name: "Test".to_string(),
            message: Some("Hi".to_string()),
        };
        let context = ComponentContext::new();

        let result = greeting.render(&props, &context);
        assert!(result.is_ok());

        let html = result.unwrap();
        let rendered = html.render();
        assert!(rendered.contains("Hi, Test!"));
        assert!(rendered.contains("Welcome to RUITL!"));
    }

    #[test]
    fn test_layout_component() {
        let layout = Layout;
        let context = ComponentContext::new();

        let result = layout.render(&EmptyProps, &context);
        assert!(result.is_ok());

        let html = result.unwrap();
        let rendered = html.render();
        assert!(rendered.contains("<html>"));
        assert!(rendered.contains("RUITL Hello World"));
        assert!(rendered.contains("RUITL Demo"));
    }
}
