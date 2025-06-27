//! Server Integration Example for RUITL
//!
//! This example demonstrates how to integrate RUITL-generated components with an HTTP server
//! to serve dynamic HTML pages to browsers.
//!
//! Run with: cargo run --example server_integration
//! Then visit: http://localhost:3000

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use ruitl::prelude::*;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio;

// Example generated components (in real usage, these come from .ruitl files)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageProps {
    pub title: String,
    pub content: String,
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
                            "body { font-family: Arial, sans-serif; margin: 40px; }
                             .container { max-width: 800px; margin: 0 auto; }
                             .button { background: #007bff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }
                             .card { border: 1px solid #ddd; padding: 20px; margin: 20px 0; border-radius: 8px; }"
                        ))),
                ))
                .child(Html::Element(
                    body().child(Html::Element(
                        div()
                            .class("container")
                            .child(Html::Element(h1().text(&props.title)))
                            .child(Html::Element(div().text(&props.content))),
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

        if let Some(href) = &props.href {
            Ok(Html::Element(
                a().attr("href", href)
                    .class(&format!("button btn-{}", props.variant))
                    .text(&props.text),
            ))
        } else {
            Ok(Html::Element(
                button()
                    .class(&format!("button btn-{}", props.variant))
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
        Ok(Html::Element(
            div()
                .class("card")
                .child(Html::Element(h3().text(&format!("User: {}", props.name))))
                .child(Html::Element(p().text(&format!("Email: {}", props.email))))
                .child(Html::Element(p().text(&format!("Role: {}", props.role)))),
        ))
    }
}

// HTTP Handler Functions
async fn handle_request(req: Request<Body>) -> std::result::Result<Response<Body>, Infallible> {
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => serve_home_page().await,
        (&Method::GET, "/users") => serve_users_page().await,
        (&Method::GET, "/about") => serve_about_page().await,
        (&Method::GET, "/api/users") => serve_users_api().await,
        _ => serve_404().await,
    };

    Ok(response)
}

async fn serve_home_page() -> Response<Body> {
    let page = Page;
    let props = PageProps {
        title: "RUITL Server Integration Demo".to_string(),
        content: "Welcome to the RUITL server integration example!".to_string(),
    };

    match render_full_page(page, props).await {
        Ok(html) => {
            let body = add_navigation(&html);
            Response::builder()
                .header("content-type", "text/html")
                .body(Body::from(body))
                .unwrap()
        }
        Err(e) => error_response(&format!("Render error: {}", e)),
    }
}

async fn serve_users_page() -> Response<Body> {
    let users = vec![
        UserCardProps {
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            role: "Admin".to_string(),
        },
        UserCardProps {
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            role: "User".to_string(),
        },
        UserCardProps {
            name: "Carol Davis".to_string(),
            email: "carol@example.com".to_string(),
            role: "Moderator".to_string(),
        },
    ];

    let context = ComponentContext::new();
    let user_card = UserCard;

    let mut users_html = String::new();
    for user_props in users {
        match user_card.render(&user_props, &context) {
            Ok(html) => users_html.push_str(&html.render()),
            Err(e) => return error_response(&format!("User render error: {}", e)),
        }
    }

    let page = Page;
    let props = PageProps {
        title: "Users - RUITL Demo".to_string(),
        content: format!("<h2>Our Users</h2>{}", users_html),
    };

    match render_full_page(page, props).await {
        Ok(html) => {
            let body = add_navigation(&html);
            Response::builder()
                .header("content-type", "text/html")
                .body(Body::from(body))
                .unwrap()
        }
        Err(e) => error_response(&format!("Render error: {}", e)),
    }
}

async fn serve_about_page() -> Response<Body> {
    let button = Button;
    let button_props = ButtonProps {
        text: "Back to Home".to_string(),
        variant: "primary".to_string(),
        href: Some("/".to_string()),
    };

    let context = ComponentContext::new();
    let button_html = match button.render(&button_props, &context) {
        Ok(html) => html.render(),
        Err(e) => return error_response(&format!("Button render error: {}", e)),
    };

    let page = Page;
    let props = PageProps {
        title: "About - RUITL Demo".to_string(),
        content: format!(
            "<h2>About RUITL</h2>
             <p>RUITL is a Rust UI Template Language that compiles templates to efficient Rust code.</p>
             <p>This demo shows server-side rendering with generated components.</p>
             <h3>Features:</h3>
             <ul>
                 <li>✅ Type-safe components</li>
                 <li>✅ Server-side rendering</li>
                 <li>✅ Zero runtime template parsing</li>
                 <li>✅ HTML escaping built-in</li>
             </ul>
             <p>{}</p>",
            button_html
        ),
    };

    match render_full_page(page, props).await {
        Ok(html) => {
            let body = add_navigation(&html);
            Response::builder()
                .header("content-type", "text/html")
                .body(Body::from(body))
                .unwrap()
        }
        Err(e) => error_response(&format!("Render error: {}", e)),
    }
}

async fn serve_users_api() -> Response<Body> {
    let users = vec![
        UserCardProps {
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            role: "Admin".to_string(),
        },
        UserCardProps {
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            role: "User".to_string(),
        },
    ];

    let json = serde_json::to_string(&users).unwrap();
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn serve_404() -> Response<Body> {
    let page = Page;
    let props = PageProps {
        title: "404 - Page Not Found".to_string(),
        content: "<h2>Page Not Found</h2><p>The requested page could not be found.</p>".to_string(),
    };

    match render_full_page(page, props).await {
        Ok(html) => {
            let body = add_navigation(&html);
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("content-type", "text/html")
                .body(Body::from(body))
                .unwrap()
        }
        Err(e) => error_response(&format!("Render error: {}", e)),
    }
}

// Helper Functions
async fn render_full_page(page: Page, props: PageProps) -> ruitl::error::Result<String> {
    let context = ComponentContext::new();
    let html = page.render(&props, &context)?;
    Ok(html.render())
}

fn add_navigation(content: &str) -> String {
    let nav = r#"
        <style>
            .nav { background: #f8f9fa; padding: 10px 0; margin-bottom: 20px; border-bottom: 1px solid #ddd; }
            .nav ul { list-style: none; margin: 0; padding: 0; display: flex; justify-content: center; }
            .nav li { margin: 0 20px; }
            .nav a { text-decoration: none; color: #007bff; font-weight: bold; }
            .nav a:hover { text-decoration: underline; }
        </style>
        <nav class="nav">
            <ul>
                <li><a href="/">Home</a></li>
                <li><a href="/users">Users</a></li>
                <li><a href="/about">About</a></li>
                <li><a href="/api/users">API</a></li>
            </ul>
        </nav>
    "#;

    // Insert navigation after <body> tag
    content.replace("<body>", &format!("<body>{}", nav))
}

fn error_response(message: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "text/plain")
        .body(Body::from(format!("Error: {}", message)))
        .unwrap()
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Starting RUITL server integration demo...");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("🌐 Server running at http://{}", addr);
    println!("📄 Routes available:");
    println!("   • http://localhost:3000/       - Home page");
    println!("   • http://localhost:3000/users  - Users page with components");
    println!("   • http://localhost:3000/about  - About page with buttons");
    println!("   • http://localhost:3000/api/users - JSON API");
    println!();
    println!("✨ This demonstrates:");
    println!("   ✅ Server-side rendering with RUITL components");
    println!("   ✅ Type-safe props and validation");
    println!("   ✅ Component composition and reuse");
    println!("   ✅ HTML generation with proper escaping");
    println!("   ✅ Integration with HTTP frameworks");
    println!();
    println!("Press Ctrl+C to stop the server");

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_component_rendering() {
        let page = Page;
        let props = PageProps {
            title: "Test Page".to_string(),
            content: "Test content".to_string(),
        };

        let result = render_full_page(page, props).await;
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains("Test Page"));
        assert!(html.contains("Test content"));
        assert!(html.contains("<!DOCTYPE html") || html.contains("<html"));
    }

    #[test]
    fn test_button_validation() {
        let valid_props = ButtonProps {
            text: "Click me".to_string(),
            variant: "primary".to_string(),
            href: None,
        };
        assert!(valid_props.validate().is_ok());

        let invalid_props = ButtonProps {
            text: "Click me".to_string(),
            variant: "invalid".to_string(),
            href: None,
        };
        assert!(invalid_props.validate().is_err());
    }

    #[test]
    fn test_user_validation() {
        let valid_props = UserCardProps {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "User".to_string(),
        };
        assert!(valid_props.validate().is_ok());

        let invalid_props = UserCardProps {
            name: "".to_string(),
            email: "invalid-email".to_string(),
            role: "User".to_string(),
        };
        assert!(invalid_props.validate().is_err());
    }
}
