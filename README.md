# RUITL - Rust UI Template Language

[![Crates.io](https://img.shields.io/crates/v/ruitl.svg)](https://crates.io/crates/ruitl)
[![Documentation](https://docs.rs/ruitl/badge.svg)](https://docs.rs/ruitl)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

A modern, type-safe template engine for building HTML components and applications in Rust. RUITL combines the performance and safety of Rust with the flexibility of component-based UI development.

## ✨ Features

- 🚀 **Server-side rendering**: Deploy as serverless functions, Docker containers, or standard Rust programs
- ⚡ **Static site generation**: Create static HTML files for any hosting service
- 🦀 **Compiled components**: Components are compiled into performant Rust code
- 🔧 **Pure Rust**: Call any Rust code and use standard `if`, `match`, and `for` statements
- 🚫 **No JavaScript**: Does not require any client or server-side JavaScript
- 🎯 **Great DX**: Ships with IDE autocompletion and type safety
- 🔥 **Hot reload**: Fast development with automatic reloading
- 📦 **Component-based**: Create reusable UI components with props and lifecycle methods

## 🚀 Quick Start

### Installation

Add RUITL to your `Cargo.toml`:

```toml
[dependencies]
ruitl = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Hello World

```rust
use ruitl::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GreetingProps {
    name: String,
}

impl ComponentProps for GreetingProps {}

#[derive(Debug)]
struct Greeting;

impl Component for Greeting {
    type Props = GreetingProps;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        Ok(html! {
            <div class="greeting">
                <h1>Hello, {props.name}!</h1>
                <p>Welcome to RUITL!</p>
            </div>
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let component = Greeting;
    let props = GreetingProps {
        name: "World".to_string(),
    };
    
    let html = component.render(&props, &ComponentContext::new())?;
    println!("{}", html.render());
    
    Ok(())
}
```

## 📖 Documentation

### Creating Components

Components in RUITL are Rust structs that implement the `Component` trait:

```rust
use ruitl::prelude::*;

#[derive(Debug, Clone)]
struct ButtonProps {
    text: String,
    variant: String,
    disabled: bool,
}

impl ComponentProps for ButtonProps {}

#[derive(Debug)]
struct Button;

impl Component for Button {
    type Props = ButtonProps;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let classes = format!("btn btn-{}", props.variant);
        
        Ok(Html::Element(
            ruitl::html::button()
                .class(&classes)
                .attr("disabled", if props.disabled { "true" } else { "" })
                .text(&props.text)
        ))
    }
}
```

### HTML Generation

RUITL provides a fluent API for creating HTML:

```rust
use ruitl::html::*;

let html = div()
    .class("container")
    .id("main")
    .child(
        h1().text("Welcome")
    )
    .child(
        p().text("This is a paragraph")
    )
    .child(
        ul()
            .child(li().text("Item 1"))
            .child(li().text("Item 2"))
            .child(li().text("Item 3"))
    );
```

### Routing

Set up routes for your application:

```rust
use ruitl::router::*;

let router = Router::builder()
    .add("/")
        .get()
        .function(|ctx| {
            Ok(RouteResponse::html(
                Html::Element(div().text("Home Page"))
            ))
        })
    .add("/users/:id")
        .get()
        .function(|ctx| {
            let user_id = ctx.params.get("id").unwrap_or("unknown");
            Ok(RouteResponse::html(
                Html::Element(div().text(&format!("User: {}", user_id)))
            ))
        })
    .build();
```

### Server-Side Rendering

Deploy your RUITL application as a web server:

```rust
use ruitl::server::DevServer;
use ruitl::config::{DevConfig, RuitlConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let dev_config = DevConfig {
        port: 3000,
        host: "localhost".to_string(),
        hot_reload: true,
        ..Default::default()
    };
    
    let project_config = RuitlConfig::default();
    let mut server = DevServer::new(dev_config, project_config)?;
    
    server.start().await?;
    
    Ok(())
}
```

### Static Site Generation

Generate static HTML files:

```rust
use ruitl::static_gen::StaticGenerator;
use ruitl::config::{StaticConfig, RuitlConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let static_config = StaticConfig {
        base_url: "https://mysite.com".to_string(),
        routes: vec!["/".to_string(), "/about".to_string()],
        ..Default::default()
    };
    
    let project_config = RuitlConfig::default();
    let generator = StaticGenerator::new(static_config, project_config)?;
    
    let stats = generator.generate(Path::new("dist")).await?;
    println!("Generated {} pages", stats.pages_generated);
    
    Ok(())
}
```

## 🛠️ CLI Tools

RUITL comes with a powerful CLI for development and deployment:

### Create a New Project

```bash
ruitl new my-app
cd my-app
```

### Start Development Server

```bash
ruitl dev
# Server starts at http://localhost:3000 with hot reload
```

### Build for Production

```bash
ruitl build --target web --minify
```

### Generate Static Site

```bash
ruitl static --out-dir dist --base-url https://mysite.com
```

### Add Components

```bash
ruitl add component Button
ruitl add page About
ruitl add layout MainLayout
```

## 📁 Project Structure

```
my-ruitl-app/
├── src/
│   ├── main.rs
│   ├── components/
│   │   ├── button.rs
│   │   └── navbar.rs
│   └── pages/
│       ├── index.rs
│       └── about.rs
├── templates/
│   ├── base.html
│   └── layout.html
├── static/
│   ├── style.css
│   └── images/
├── ruitl.toml
└── Cargo.toml
```

## ⚙️ Configuration

Configure your project with `ruitl.toml`:

```toml
[project]
name = "my-app"
version = "0.1.0"
description = "My RUITL application"

[build]
src_dir = "src"
out_dir = "dist"
minify = true
source_maps = false

[dev]
port = 3000
host = "localhost"
hot_reload = true
open = true

[static]
base_url = "/"
generate_sitemap = true
generate_robots = true

[ssr]
port = 8080
host = "0.0.0.0"
cache = true
```

## 🎯 Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN ruitl build --target server --mode release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/dist/server /usr/local/bin/
EXPOSE 8080
CMD ["server"]
```

### Serverless (AWS Lambda)

```bash
ruitl build --target serverless
# Deploy dist/lambda.zip to AWS Lambda
```

### Static Hosting

```bash
ruitl static --out-dir dist
# Upload dist/ folder to any static host (Netlify, Vercel, S3, etc.)
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

- Inspired by modern web frameworks like Next.js and SvelteKit
- Built on the shoulders of the amazing Rust ecosystem
- Special thanks to the Rust community for continuous support

## 📊 Benchmarks

RUITL is designed for performance:

- **Server-side rendering**: ~10x faster than Node.js alternatives
- **Static generation**: Processes 1000+ pages in seconds
- **Memory usage**: Minimal footprint with Rust's zero-cost abstractions
- **Bundle size**: No JavaScript runtime = smaller bundles

## 🔮 Roadmap

- [ ] WebAssembly client-side hydration
- [ ] CSS-in-Rust styling system
- [ ] Database integration helpers
- [ ] Form handling utilities
- [ ] Internationalization support
- [ ] Plugin ecosystem
- [ ] Visual component editor

---

**Made with ❤️ and 🦀 by the RUITL team**