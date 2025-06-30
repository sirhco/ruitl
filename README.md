# RUITL - Rust UI Template Language v0.1.0

[![Crates.io](https://img.shields.io/crates/v/ruitl.svg)](https://crates.io/crates/ruitl)
[![Documentation](https://docs.rs/ruitl/badge.svg)](https://docs.rs/ruitl)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

> **⚠️ Status: Experimental (v0.1.0)** - This project is experimental and not production ready. Template compilation, CLI, and component generation are working but may have breaking changes in future versions.

A modern template compiler for building type-safe HTML components in Rust. RUITL compiles `.ruitl` template files into efficient Rust code at build time, providing the safety and performance of Rust with a natural HTML-like syntax.

## ✨ Key Features

- 🔄 **Template Compilation**: `.ruitl` files compiled to Rust code at build time
- 🦀 **Type Safety**: Generated components with full Rust type checking
- ⚡ **Zero Runtime**: Templates compiled away - pure Rust performance
- 🔧 **Cargo Integration**: Seamless build process with standard Rust tooling
- 📦 **Component Props**: Type-safe props with validation and defaults
- 🎯 **HTML Generation**: Clean, efficient HTML output
- 🚫 **No JavaScript**: Pure Rust, server-side rendering focus

## 🚀 Quick Start

You can get started with RUITL in three ways:

### Option 1: Explore the RUITL Repository (Development)

Clone and explore the RUITL repository with built-in examples:

```bash
# Clone RUITL repository
git clone https://github.com/sirhco/ruitl.git
cd ruitl

# Build RUITL
cargo build

# Compile example templates in the repository
cargo run -- compile

# Run the server integration example with live templates
cargo run --example server_integration
# Server available at http://localhost:3000
```

This gives you immediate access to working examples and lets you experiment with RUITL templates directly in the repository.

### Option 2: Use the Project Scaffolder (Recommended)

Create a complete project with examples and server support:

```bash
# Clone RUITL repository
git clone <repository>
cd ruitl

# Create a new project with server and examples
cargo run -- scaffold --name my-project --with-server --with-examples

# Navigate to your new project
cd my-project

# Compile templates and run (using included RUITL binary)
cargo run --bin ruitl -- compile
cargo run
# Server available at http://localhost:3000
```

This creates a complete project structure with example components, HTTP server, static assets, and documentation.

### Option 3: Manual Setup

### 1. Add RUITL to Your Project

```toml
# Cargo.toml
[dependencies]
ruitl = "0.1.0"
tokio = { version = "1.0", features = ["full"] }

[build-dependencies]
walkdir = "2.3"
```

### 2. Create a Template

Create `templates/Button.ruitl`:

```ruitl
// Button.ruitl - A reusable button component
component Button {
    props {
        text: String,
        variant: String = "primary",
        disabled: bool = false,
    }
}

ruitl Button(text: String, variant: String) {
    <button class={format!("btn btn-{}", variant)} type="button">
        {text}
    </button>
}
```

### 3. Use Generated Components

The build process automatically generates Rust components:

```rust
use ruitl::prelude::*;

// Generated components are available after build
// mod generated;
// use generated::*;

fn main() -> Result<()> {
    // Component instances
    let button = Button;

    // Type-safe props
    let props = ButtonProps {
        text: "Click Me!".to_string(),
        variant: "primary".to_string(),
    };

    // Render to HTML
    let context = ComponentContext::new();
    let html = button.render(&props, &context)?;

    println!("{}", html.render());
    // Output: <button class="btn btn-primary" type="button">Click Me!</button>

    Ok(())
}
```

## 🏗️ Project Scaffolding

RUITL includes a powerful project scaffolder that creates complete project structures with examples and server implementations.

### Creating a New Project

```bash
# Create a basic RUITL project
cargo run -- scaffold --name my-project

# Create a project with HTTP server support
cargo run -- scaffold --name my-project --with-server

# Create a project with example components
cargo run -- scaffold --name my-project --with-examples

# Create a full-featured project with both server and examples
cargo run -- scaffold --name my-project --with-server --with-examples

# Specify target directory
cargo run -- scaffold --name my-project --target ./projects --with-server
```

### Generated Project Structure

The scaffolder creates a complete project structure:

```
my-ruitl-project/
├── .gitignore             # Git ignore file
├── Cargo.toml             # Project configuration with dependencies
├── README.md              # Project documentation
├── ruitl.toml             # RUITL-specific configuration
├── bin/
│   └── ruitl.rs           # Included RUITL CLI binary
├── src/
│   ├── main.rs            # Main application (server if --with-server)
│   ├── lib.rs             # Library code
│   └── handlers/          # HTTP handlers (if --with-server)
│       └── mod.rs
├── templates/             # RUITL template files
│   ├── Button.ruitl       # Example button component
│   ├── Card.ruitl         # Example card component
│   └── Layout.ruitl       # Example layout component
├── static/                # Static assets
│   ├── css/
│   │   └── styles.css     # Complete CSS framework
│   └── js/
│       └── main.js        # Interactive JavaScript
├── generated/             # Generated Rust code (created after compile)
│   ├── Button.rs          # Generated from Button.ruitl
│   ├── Card.rs            # Generated from Card.ruitl
│   ├── Layout.rs          # Generated from Layout.ruitl
│   └── mod.rs             # Module exports
└── examples/              # Additional examples (if --with-examples)
```

**Note**: The `generated/` directory is created and populated when you run `cargo run --bin ruitl -- compile`. It contains the Rust code generated from your `.ruitl` template files.

**Self-Contained Binary**: Each scaffolded project includes its own RUITL CLI binary in the `bin/` directory, so you don't need to install RUITL globally. All template compilation is done using `cargo run --bin ruitl -- <command>`.

### Server Implementation Features

When using `--with-server`, the scaffolder generates:

- **Complete HTTP Server**: Built with Tokio and Hyper
- **Routing System**: Clean URL routing with static file serving
- **Component Integration**: Server-side rendering with RUITL components
- **Static Assets**: CSS and JavaScript served efficiently
- **Error Handling**: 404 pages and error responses
- **Development Ready**: Ready to run with `cargo run`

Example server routes:
- `http://localhost:3000/` - Home page with welcome content
- `http://localhost:3000/about` - About page with project info
- `http://localhost:3000/static/*` - Static file serving

### Example Components

With `--with-examples`, you get three complete example components:

**Button Component** (`templates/Button.ruitl`):
```ruitl
component Button {
    props {
        text: String,
        variant: String = "primary",
        size: String = "medium",
        disabled: bool = false,
        onclick: String?,
    }
}

ruitl Button(props: ButtonProps) {
    <button
        class={format!("btn btn-{} btn-{}", props.variant, props.size)}
        disabled?={props.disabled}
        onclick={props.onclick.as_deref().unwrap_or("")}
        type="button"
    >
        {props.text}
    </button>
}
```

**Card Component** with conditional rendering and **Layout Component** with full HTML structure are also included.

### Getting Started with Scaffolded Project

After scaffolding:

```bash
# Navigate to your project
cd my-project

# Compile RUITL templates (using included binary)
cargo run --bin ruitl -- compile

# Build the project
cargo build

# Run the server (if --with-server was used)
cargo run
# Server starts at http://localhost:3000

# Or run as library (if no server)
cargo run
```

**Why the included binary?** Each scaffolded project includes its own RUITL CLI wrapper (`bin/ruitl.rs`) that uses the same RUITL version as your project dependencies. This ensures version consistency and eliminates the need for global RUITL installation.

### Scaffold Command Options

| Option | Description | Default |
|--------|-------------|---------|
| `--name <NAME>` | Project name | `my-ruitl-project` |
| `--target <PATH>` | Target directory | `.` (current directory) |
| `--with-server` | Include HTTP server implementation | `false` |
| `--with-examples` | Include example components | `false` |
| `--verbose` | Verbose output | `false` |

### Dependencies Added

The scaffolder automatically configures appropriate dependencies:

**Basic Project**:
- `ruitl` - Template engine
- `serde` - Serialization
- `anyhow` - Error handling

**With Server** (adds):
- `tokio` - Async runtime
- `hyper` - HTTP server
- `serde_json` - JSON handling

## 🖥️ CLI Commands

RUITL provides a comprehensive command-line interface for project management and template compilation.

### Installation

```bash
# Clone and build RUITL
git clone <repository>
cd ruitl
cargo build --release

# Use via cargo run for development
cargo run -- <command>

# Or install globally (after publishing)
cargo install ruitl
ruitl <command>

# In scaffolded projects, use the included binary
cargo run --bin ruitl -- <command>
```

### Available Commands

#### `scaffold` - Create New Projects

Create a new RUITL project with optional server and examples:

```bash
# Basic usage
ruitl scaffold --name my-project

# Full options
ruitl scaffold \
  --name my-project \
  --target ./projects \
  --with-server \
  --with-examples \
  --verbose
```

**Options:**
- `--name <NAME>` - Project name (default: `my-ruitl-project`)
- `--target <PATH>` - Target directory (default: current directory)
- `--with-server` - Include HTTP server implementation
- `--with-examples` - Include example components
- `--verbose` - Show detailed output

#### `compile` - Compile Templates

Compile `.ruitl` template files to Rust code:

```bash
# Basic compilation
ruitl compile

# Specify directories
ruitl compile --src-dir templates --out-dir generated

# Watch mode for development
ruitl compile --watch

# Full options
ruitl compile \
  --src-dir ./templates \
  --out-dir ./generated \
  --watch \
  --verbose
```

**Options:**
- `--src-dir <PATH>` - Template source directory (default: `templates`)
- `--out-dir <PATH>` - Generated code output directory (default: `generated`)
- `--watch` - Watch for file changes and recompile automatically
- `--verbose` - Show detailed compilation output

#### `version` - Show Version

Display RUITL version information:

```bash
ruitl version
```

### Global Options

Available for all commands:

- `--config <PATH>` - Custom configuration file path
- `--env <ENV>` - Environment setting (default: `development`)
- `--verbose` - Enable verbose output
- `--help` - Show command help

### Configuration File

Create `ruitl.toml` in your project root:

```toml
[project]
name = "my-project"
version = "0.1.0"
description = "My RUITL project"
authors = ["Your Name <your.email@example.com>"]

[build]
template_dir = "templates"
out_dir = "generated"
src_dir = "src"

[server]
host = "127.0.0.1"
port = 3000
static_dir = "static"

[dev]
watch = true
hot_reload = false
```

### Development Workflow

#### Working with Scaffolded Projects

For projects created with the scaffolder:

```bash
# 1. Create new project
ruitl scaffold --name my-app --with-server --with-examples

# 2. Navigate to project
cd my-app

# 3. Start development with watch mode (using included binary)
cargo run --bin ruitl -- compile --watch &

# 4. Run the server (in another terminal)
cargo run

# 5. Edit templates and see changes automatically
# Templates in templates/ are watched and recompiled
```

#### Working on RUITL Repository

For contributing to RUITL or using repository examples:

```bash
# 1. Clone and build RUITL
git clone https://github.com/sirhco/ruitl.git
cd ruitl
cargo build

# 2. Compile repository templates
cargo run -- compile

# 3. Run examples with live reload
cargo run --example server_integration
# Visit http://localhost:3000

# 4. For development with watch mode
cargo run -- compile --watch &

# 5. Edit templates/ files and see changes in examples
```

### Examples

```bash
# Create a simple library project
ruitl scaffold --name ui-components --with-examples

# Create a full web application
ruitl scaffold --name my-webapp --with-server --with-examples --target ./projects

# Compile templates with custom paths
ruitl compile --src-dir ./my-templates --out-dir ./src/generated

# Development workflow with file watching
ruitl compile --watch --verbose
```

## 📝 Template Syntax

### Component Definitions

Define reusable components with type-safe props:

```ruitl
component UserCard {
    props {
        name: String,
        email: String,
        role: String = "user",
        avatar_url: String?,
        is_verified: bool = false,
    }
}
```

### Template Implementation

Implement the component's HTML structure:

```ruitl
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
}
```

### Expression Interpolation

Use Rust expressions directly in templates:

```ruitl
ruitl Example(count: u32, items: Vec<String>) {
    <div>
        <h1>Items ({count})</h1>
        <p>Status: {if count > 0 { "Has items" } else { "Empty" }}</p>
        <p>First item: {items.first().unwrap_or(&"None".to_string())}</p>
    </div>
}
```

## ⚙️ Build Process

RUITL integrates seamlessly with Cargo's build system:

### Project Structure

```
my-app/
├── Cargo.toml
├── build.rs                 # Auto-compile templates
├── src/
│   ├── main.rs
│   └── lib.rs
└── templates/              # Your .ruitl files
    ├── Button.ruitl
    ├── UserCard.ruitl
    └── Layout.ruitl
```

### Build Integration

Add to your `build.rs`:

```rust
// build.rs
fn main() {
    // RUITL templates automatically compiled
    println!("cargo:rerun-if-changed=templates");
}
```

### Generated Code

Templates compile to efficient Rust code:

```rust
// Generated from Button.ruitl
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ButtonProps {
    pub text: String,
    pub variant: String, // default: "primary"
}

impl ComponentProps for ButtonProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Button;

impl Component for Button {
    type Props = ButtonProps;

    fn render(&self, props: &Self::Props, context: &ComponentContext) -> ruitl::error::Result<Html> {
        Ok(html! {
            <button class={format!("btn btn-{}", props.variant)} type="button">
                {props.text}
            </button>
        })
    }
}
```

## 🧪 Examples

### Scaffolded Project Examples

The best way to explore RUITL is through the project scaffolder:

```bash
# Clone RUITL repository
git clone <repository>
cd ruitl

# Create a complete example project
cargo run -- scaffold --name ruitl-demo --with-server --with-examples

# Navigate and run the example
cd ruitl-demo
cargo run --bin ruitl -- compile
cargo run
# Visit http://localhost:3000
```

This creates a complete project with:
- **Three example components**: Button, Card, and Layout
- **HTTP server**: Complete web application
- **Static assets**: CSS framework and JavaScript
- **Multiple pages**: Home, About, and 404 error pages
- **Type-safe props**: Demonstrates all RUITL features

### Component Examples from Scaffolded Project

**Button Component** - Shows props with defaults:
```rust
// Usage in Rust code
let button = Button;
let props = ButtonProps {
    text: "Click Me!".to_string(),
    variant: "primary".to_string(),
    size: "medium".to_string(),
    disabled: false,
    onclick: Some("handleClick()".to_string()),
};
let html = button.render(&props, &context)?;
```

**Card Component** - Shows conditional rendering:
```rust
// Usage with optional footer
let card = Card;
let props = CardProps {
    title: "Welcome".to_string(),
    content: "This is a card component with conditional footer.".to_string(),
    footer: Some("Card footer text".to_string()),
    variant: "default".to_string(),
};
```

**Layout Component** - Shows full page structure:
```rust
// Complete page layout
let layout = Layout;
let props = LayoutProps {
    title: "My App".to_string(),
    description: Some("A RUITL application".to_string()),
    children: "<div>Page content here</div>".to_string(),
};
```

### Repository Examples

The RUITL repository includes several built-in examples you can run immediately:

```bash
# Clone and build RUITL
git clone https://github.com/sirhco/ruitl.git
cd ruitl
cargo build

# Run the server integration example (recommended)
cargo run --example server_integration
# Visit http://localhost:3000 to see:
# - Live component rendering
# - Multiple page routing
# - Static asset serving
# - Type-safe component usage

# Other examples available:
cargo run --example basic_usage           # Basic component usage
cargo run --example hello_world          # Simple hello world
cargo run --example html_output_demo     # HTML generation demo
cargo run --example template_compiler_demo # Template compilation demo
cargo run --example advanced_features_demo # Advanced RUITL features
```

**Server Integration Example Features:**
- **Live Components**: See Button, UserCard, and Page components in action
- **Multiple Routes**: Home, Users, About, and API endpoints
- **Server-Side Rendering**: Components rendered to HTML
- **Type Safety**: Demonstrates prop validation and error handling
- **Navigation**: Working page navigation with styled components

### Original Demo

You can also run the original template compiler demo:

```bash
cargo run --example template_compiler_demo
- Template syntax examples
- Generated code structure
- Build process workflow
- Component usage patterns

## 🛠️ Development Workflow

1. **Write Templates**: Create `.ruitl` files in `templates/` directory
2. **Build**: Run `cargo build` to compile templates
3. **Import**: Use generated components in your Rust code
4. **Iterate**: Templates recompile automatically on changes

```bash
# Create new template
echo 'component Hello { props { name: String } }
ruitl Hello(name: String) { <h1>Hello, {name}!</h1> }' > templates/Hello.ruitl

# Compile
cargo build

# Use in your code
# let hello = Hello;
# let props = HelloProps { name: "World".to_string() };
# let html = hello.render(&props, &context)?;
```

## 📊 Current Status

### ✅ Working Features

- [x] Build script template compilation
- [x] CLI template compilation
- [x] Basic template syntax (components, props, templates)
- [x] Advanced template syntax (conditionals, loops, composition)
- [x] Type-safe props with defaults and validation
- [x] Expression interpolation with complex Rust expressions
- [x] HTML element generation (all standard elements)
- [x] Component trait implementation
- [x] Cargo integration
- [x] Conditional rendering (`if/else` statements)
- [x] Loop rendering (`for` loops over iterables)
- [x] Component composition (`@Component` syntax)
- [x] Pattern matching (`match` expressions)
- [x] Import statements
- [x] Boolean and primitive type operations
- [x] Complex nested template structures

### 🚧 Enhancement Opportunities

- [ ] Hot reload development mode
- [ ] IDE support and syntax highlighting
- [ ] Advanced error messages with suggestions
- [ ] Template inheritance
- [ ] Performance optimizations

### 🎯 Roadmap

- [x] ~~Advanced template features~~ **COMPLETE**
- [ ] Hot reload development mode
- [ ] IDE support and syntax highlighting
- [ ] Performance optimizations and caching
- [ ] Template inheritance
- [ ] Server-side streaming
- [ ] Component testing utilities
- [ ] Template debugging tools

## 🔧 Configuration

Configure template compilation in your `Cargo.toml`:

```toml
[package.metadata.ruitl]
template_dir = "templates"
generated_dir = "generated"
```

## 🤔 FAQ

**Q: How does RUITL compare to other templating solutions?**
A: RUITL compiles templates to native Rust code at build time, providing zero runtime overhead and full type safety.

**Q: Can I use existing Rust code in templates?**
A: Yes! Templates support arbitrary Rust expressions and function calls.

**Q: Is RUITL production ready?**
A: Yes! All core and advanced features are working, including conditionals, loops, and component composition. Ready for production use.

**Q: How does performance compare to runtime templating?**
A: Since templates compile to native Rust code, performance is excellent with no template parsing overhead.

## 🤝 Contributing

We welcome contributions! Areas where help is needed:

- Hot reload development mode
- IDE support and syntax highlighting
- Improving error messages and suggestions
- Performance optimizations
- Writing documentation and guides
- Creating advanced examples
- Template testing utilities

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for detailed status.

## 📝 License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🙏 Acknowledgments

- Inspired by [Templ](https://templ.guide/) for Go
- Built with the amazing Rust ecosystem
- Thanks to early contributors and testers

---

**RUITL: Compile-time templates for Rust 🦀**

*Want to contribute? Check out our [issues](https://github.com/sirhco/ruitl/issues) or start with the [implementation status](IMPLEMENTATION_STATUS.md).*
