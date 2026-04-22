# RUITL - Rust UI Template Language v0.2.0

[![Crates.io](https://img.shields.io/crates/v/ruitl.svg)](https://crates.io/crates/ruitl)
[![Documentation](https://docs.rs/ruitl/badge.svg)](https://docs.rs/ruitl)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

> **⚠️ Status: Pre-release (v0.2.0)** — Core feature set stable. Breaking changes possible before v1.0 while SSG and ergonomics settle.

A template compiler for building type-safe HTML components in Rust, modelled on Go's [templ](https://templ.guide). RUITL compiles `.ruitl` template files into sibling `*_ruitl.rs` files (committed to source control, reviewable in diffs) and links them into your binary at build time.

## ✨ Key Features

- 🔄 **Template Compilation**: `.ruitl` → sibling `*_ruitl.rs` (templ-style `_templ.go` convention)
- 🦀 **Type Safety**: Generated components with full Rust type checking
- ⚡ **Zero Runtime Parsing**: Templates compiled away, pure Rust at render
- 🔧 **Cargo Integration**: `build.rs` and `ruitl` CLI share one compiler
- 📦 **Component Props**: Type-safe props with validation, defaults, generics
- ♻️ **Incremental Builds**: Hash-based skip when `.ruitl` source is unchanged
- 👀 **Watch Mode**: `ruitl compile --watch` auto-recompiles on save
- 🎯 **HTML Generation**: Clean, deterministic, attribute-order-stable output
- 🚫 **No JavaScript**: Pure Rust, server-side rendering focus

## Status

| Feature | State | Notes |
|---|---|---|
| Template parser | Stable | components, props, `if`/`for`/`match`, composition `@X(...)`, imports |
| Generics | Stable (type params) | `<T, U: Bound>`. Lifetime params rejected with explicit error |
| Codegen | Stable | Deterministic attribute order; prop bindings emitted only when referenced |
| Incremental build | Stable | `// ruitl-hash:` header skip; `CODEGEN_VERSION` cache-buster |
| Watch mode | Stable (dev feature) | `hotwatch`-backed; 150ms debounce |
| Scaffolder | Stable | `ruitl scaffold` emits sibling-file projects with `bin/ruitl.rs` wrapper |
| Snapshot tests | Stable | `insta` + `prettyplease`; fixtures in `tests/fixtures/snapshots/` |
| Minification | Optional | `--features minify` post-render via `minify-html` (planned) |
| Static site generation | Planned | `ruitl build` subcommand with `[[routes]]` config (planned) |
| Parser error context | Rustc-style frame | Line/col + caret + source context |
| Editor support | Stable | tree-sitter grammar + LSP w/ diagnostics, formatting, completion (`@` + `<` + prop-names inside `@X(...)`), hover, go-to-definition |
| Formatter | Stable | `ruitl fmt [--check]` CLI + LSP `textDocument/formatting`. Idempotent. Preserves leading comments. |
| Raw-HTML expression | Stable | `{!expr}` inside a template body injects the runtime value as raw HTML (no escaping). |
| Template inheritance | Stable | `@X(...) { body }` + `{children}` slot. Auto-injects `pub children: Html` on the callee's Props when the slot is used. |
| Did-you-mean errors | Stable | Codegen validation suggests closest declared component/prop name on typos via Levenshtein. |
| Parallel compile | Stable | `compile_dir_sibling` fans out with `rayon` behind the `parallel` feature (default on). |
| Buffer-reuse render | Stable | `Html::render_into(&mut String)`, `render_with_capacity`, `len_hint` for hot request loops. |
| SSR streaming | Stable | `Html::to_chunks()` splits a top-level `Fragment` for `hyper::Body::wrap_stream`. See `examples/streaming_demo.rs`. |
| Dev server | Stable (dev + server features) | `ruitl dev` watches `.ruitl`, serves SSE reload at `/ruitl/reload` so browsers auto-refresh. |
| Testing helpers | Optional (`testing` feature) | `ruitl::testing::{ComponentTestHarness, HtmlAssertion}` + `assert_html_contains!` / `assert_renders_to!`. |
| AST debug dump | Stable | `ruitl compile --emit-ast` writes a pretty-Debug of the parser AST next to each source. |

See `tests/fixtures/snapshots/*.snap` for canonical codegen output.

**Browse the gallery:** [`examples/README.md`](examples/README.md) indexes every
`.ruitl` fixture and example binary by learning goal.

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
├── templates/             # .ruitl sources AND their generated siblings (committed)
│   ├── Button.ruitl
│   ├── Button_ruitl.rs    # Generated sibling (templ-style, checked in)
│   ├── Card.ruitl
│   ├── Card_ruitl.rs
│   ├── Layout.ruitl
│   ├── Layout_ruitl.rs
│   └── mod.rs             # Auto-generated re-exports
└── examples/              # Additional examples (if --with-examples)
```

**Note**: Generated `*_ruitl.rs` files live next to their `.ruitl` sources (Go templ's `_templ.go` convention). They are reviewable and checked in. Running `cargo run --bin ruitl -- compile` or `cargo build` regenerates them.

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

Compile `.ruitl` template files to Rust code. Each `Foo.ruitl` produces a sibling `Foo_ruitl.rs` in the same directory (templ-style, checked in).

```bash
# Basic compilation (reads templates/, writes sibling *_ruitl.rs files)
ruitl compile

# Specify source directory
ruitl compile --src-dir my-templates

# Watch mode for development
ruitl compile --watch

# Full options
ruitl compile \
  --src-dir ./templates \
  --watch \
  --verbose
```

**Options:**
- `--src-dir <PATH>` - Template source directory (default: `templates`)
- `--watch` - Watch for file changes and recompile automatically
- `--verbose` - Show detailed compilation output

#### `dev` - Development Server with Browser Reload

Watch `.ruitl` files, recompile on save, and push a reload event to any
browser subscribed to the sidecar SSE endpoint. Intentionally does NOT
manage your app server process — pair it with `cargo watch -x run` or run
your app manually in another terminal.

```bash
# Default — watch ./templates, sidecar on port 35729
ruitl dev

# Custom directory and port
ruitl dev --src-dir my-templates --reload-port 40000
```

Add this script tag to your layout while in development:

```html
<script src="http://127.0.0.1:35729/ruitl/reload.js"></script>
```

**Options:**
- `--src-dir <PATH>` - Template source directory (default: `templates`)
- `--reload-port <PORT>` - Reload sidecar port (default: `35729`)

The server exposes two endpoints:

- `GET /ruitl/reload.js` — auto-reconnecting SSE client script.
- `GET /ruitl/reload` — SSE stream; fires `event: reload` after each
  successful recompile.

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

# Compile templates with custom source path
ruitl compile --src-dir ./my-templates

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

### Template Inheritance via `{children}`

Pass a body block into a component with `@Name(props) { ... }` and receive
it inside the callee with the `{children}` slot. Mirrors Go templ's
children-prop convention.

```ruitl
component Card {
    props {
        title: String,
    }
}

ruitl Card(title: String) {
    <div class="card">
        <h2>{title}</h2>
        <div class="body">{children}</div>
    </div>
}

ruitl Page() {
    @Card(title: "Welcome".to_string()) {
        <p>First paragraph.</p>
        <p>Second paragraph.</p>
    }
}
```

Codegen auto-injects `pub children: Html` into the callee's Props struct
whenever its template body references `{children}`. Call sites without a
body block default the field to `Html::Empty`. Multiple `{children}` refs
in the same template are allowed; each expands to a clone of the slot.
The bare identifier `{children}` is the slot placeholder — `{my.children}`
or any dotted path stays a normal expression.

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

- [x] Hot reload development mode (`ruitl dev` — SSE browser reload; pair with `cargo watch -x run` to restart the app binary)
- [x] IDE support and syntax highlighting (Zed + VS Code extensions, tree-sitter grammar)
- [x] Advanced error messages with suggestions (did-you-mean for unknown components/props)
- [x] Template inheritance (`{children}` slot + `@Card(...) { body }` syntax)
- [x] Performance optimizations (rayon parallel compile, buffer-reuse render API, criterion benches)

### 🎯 Roadmap

- [x] ~~Advanced template features~~ **COMPLETE**
- [x] ~~Hot reload development mode~~ **COMPLETE** (`ruitl dev`)
- [x] ~~IDE support and syntax highlighting~~ **COMPLETE**
- [x] ~~Performance optimizations and caching~~ **COMPLETE**
- [x] ~~Template inheritance~~ **COMPLETE** (`{children}` slot)
- [x] ~~Server-side streaming~~ **COMPLETE** (`Html::to_chunks` + `streaming_demo`)
- [x] ~~Component testing utilities~~ **COMPLETE** (`ruitl::testing`, `testing` feature)
- [x] ~~Template debugging tools~~ **PARTIAL** (`ruitl compile --emit-ast`)

## 🔧 Configuration

Configure template compilation in your `Cargo.toml`:

```toml
[package.metadata.ruitl]
template_dir = "templates"
generated_dir = "generated"
```

## 🖋️ Editor support

Four editor-integration crates ship alongside the compiler:

- **[`tree-sitter-ruitl`](tree-sitter-ruitl/README.md)** — tree-sitter grammar for syntax highlighting in Neovim, Helix, Zed, and any tree-sitter-aware editor. Injects the `rust` language into `{ ... }` expression spans so embedded Rust highlights too.
- **[`ruitl_lsp`](ruitl_lsp/README.md)** — Language Server. Reports parse and codegen errors as `textDocument/publishDiagnostics` in real time. Supports formatting, completion (`@` + `<` + prop-names inside `@X(...)`), hover, and go-to-definition. Install via `cargo install --path ruitl_lsp`.
- **[`zed-extension-ruitl`](zed-extension-ruitl/README.md)** — Zed extension that bundles the tree-sitter grammar and wires the LSP over stdio. Local install: `zed: install dev extension` → point at `zed-extension-ruitl/`.
- **[`vscode-extension-ruitl`](vscode-extension-ruitl/README.md)** — VS Code extension bridging `ruitl-lsp` plus a TextMate grammar fallback for syntax highlighting. Package locally with `npx vsce package && code --install-extension ruitl-0.1.0.vsix`.

Roadmap beyond this release:

- **Marketplace publishing** — Zed registry + VS Code Marketplace + npm for `tree-sitter-ruitl` (blocked on publisher account setup, not engineering).
- **Rust-aware expression completion** — completion inside `{...}` depends on a rust-analyzer bridge; intentionally out of scope.

Fallback if you don't wire the LSP: enable watch mode (`ruitl compile --watch`) in one terminal and let the parser+codegen errors from the watcher guide you.

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
