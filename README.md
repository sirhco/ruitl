# RUITL - Rust UI Template Language

[![Crates.io](https://img.shields.io/crates/v/ruitl.svg)](https://crates.io/crates/ruitl)
[![Documentation](https://docs.rs/ruitl/badge.svg)](https://docs.rs/ruitl)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

> **Status: Fully Functional** - Template compilation, CLI, advanced features, and comprehensive component generation all working!

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

Run the demo to see RUITL in action:

```bash
git clone <repository>
cd ruitl
cargo run --example template_compiler_demo
```

This example shows:
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

*Want to contribute? Check out our [issues](https://github.com/chrisolson/ruitl/issues) or start with the [implementation status](IMPLEMENTATION_STATUS.md).*