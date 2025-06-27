# RUITL Template Compilation Guide

This guide explains how to use RUITL's template compilation system to build type-safe, performant UI components using the `.ruitl` template syntax.

## Overview

RUITL's template compilation system transforms `.ruitl` template files into optimized Rust code at build time. This approach provides:

- **Natural HTML-like syntax** with Rust expressions
- **Compile-time validation** and type safety
- **Zero runtime overhead** - templates are compiled to native Rust
- **IDE support** with syntax highlighting and autocompletion
- **Component composition** with props and lifecycle management

## Architecture

```
.ruitl files → RUITL Parser → AST → Code Generator → .rs files → rustc → Binary
```

## Getting Started

### 1. Project Setup

Add RUITL to your `Cargo.toml`:

```toml
[dependencies]
ruitl = "0.1.0"
tokio = { version = "1.0", features = ["full"] }

[build-dependencies]
walkdir = "2.3"
```

Create a `build.rs` file in your project root:

```rust
// build.rs
fn main() {
    // RUITL templates will be automatically compiled
    println!("cargo:rerun-if-changed=templates");
}
```

### 2. Template Syntax

RUITL templates use a natural HTML-like syntax with Rust expressions.

#### Basic Template Structure

```ruitl
// templates/Button.ruitl

component Button {
    props {
        text: String,
        variant: String = "primary",
        disabled: bool = false,
    }
}

ruitl Button(props: ButtonProps) {
    <button
        class={format!("btn btn-{}", props.variant)}
        disabled?={props.disabled}
        type="button"
    >
        {props.text}
    </button>
}
```

#### Component Definition

Components are defined with the `component` keyword:

```ruitl
component MyComponent {
    props {
        // Required prop
        title: String,

        // Optional prop with default
        size: String = "medium",

        // Optional prop (nullable)
        description: String?,

        // Boolean with default
        visible: bool = true,

        // Complex types
        items: Vec<String>,
        metadata: HashMap<String, String>,
    }
}
```

#### Template Implementation

Templates are implemented with the `templ` keyword:

```ruitl
ruitl MyComponent(props: MyComponentProps) {
    <div class={format!("component component-{}", props.size)}>
        <h2>{props.title}</h2>

        if let Some(desc) = props.description {
            <p class="description">{desc}</p>
        }

        if props.visible {
            <ul class="items">
                for item in props.items {
                    <li class="item">{item}</li>
                }
            </ul>
        }
    </div>
}
```

### 3. Template Features

#### Expressions

Use `{}` for Rust expressions:

```ruitl
ruitl Example(props: ExampleProps) {
    <div>
        // Simple variable
        <p>{props.name}</p>

        // Method calls
        <p>{props.name.to_uppercase()}</p>

        // Complex expressions
        <p>{format!("Hello, {}!", props.name)}</p>

        // Arithmetic
        <p>Total: {props.price * props.quantity}</p>
    </div>
}
```

#### Conditional Rendering

Use `if` statements for conditional rendering:

```ruitl
ruitl ConditionalExample(props: ConditionalExampleProps) {
    <div>
        if props.show_header {
            <header>
                <h1>{props.title}</h1>
            </header>
        }

        if props.user.is_admin {
            <div class="admin-panel">Admin Controls</div>
        } else {
            <div class="user-panel">User Controls</div>
        }

        // Optional chaining
        if let Some(avatar) = props.user.avatar {
            <img src={avatar} alt="Avatar" />
        }
    </div>
}
```

#### Loops

Use `for` loops to render collections:

```ruitl
ruitl ListExample(props: ListExampleProps) {
    <ul class="list">
        for item in props.items {
            <li class="list-item">
                <span class="name">{item.name}</span>
                <span class="value">{item.value}</span>
            </li>
        }
    </ul>
}
```

#### Match Expressions

Use `match` for pattern matching:

```ruitl
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
```

#### Conditional Attributes

Use `?` for conditional attributes:

```ruitl
ruitl Input(props: InputProps) {
    <input
        type="text"
        value={props.value}
        disabled?={props.disabled}
        required?={props.required}
        placeholder={props.placeholder.unwrap_or_default()}
        class="form-input"
    />
}
```

#### Component Composition

Use `@` to invoke other components:

```ruitl
ruitl UserCard(props: UserCardProps) {
    <div class="user-card">
        <div class="user-header">
            @Avatar(
                url: props.user.avatar_url,
                name: props.user.name.clone(),
                size: "large"
            )
            <h3>{props.user.name}</h3>
        </div>

        <div class="user-actions">
            @Button(
                text: "Follow",
                variant: "primary",
                disabled: props.user.is_following
            )
            @Button(
                text: "Message",
                variant: "secondary",
                disabled: false
            )
        </div>
    </div>
}
```

### 4. Imports

Import external types and modules:

```ruitl
import "std::collections" { HashMap, Vec }
import "serde" { Serialize, Deserialize }
import "chrono" { DateTime, Utc }

component DataTable {
    props {
        data: HashMap<String, Vec<String>>,
        timestamp: DateTime<Utc>,
    }
}
```

### 5. Compilation

#### Automatic Compilation

Templates are automatically compiled during `cargo build` if you have the build script configured.

#### Manual Compilation

Use the RUITL CLI for manual compilation:

```bash
# Compile all .ruitl files in src/templates
ruitl compile --src-dir src/templates --out-dir src/generated

# Watch mode for development
ruitl compile --src-dir src/templates --out-dir src/generated --watch

# Create new template
ruitl template UserCard --type component --dir templates
```

### 6. Using Generated Components

After compilation, use the generated components in your Rust code:

```rust
// src/main.rs
use ruitl::prelude::*;

// Import generated components
mod generated;
use generated::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create component instance
    let button = Button;

    // Create props
    let props = ButtonProps {
        text: "Click me!".to_string(),
        variant: "primary".to_string(),
        disabled: false,
    };

    // Render component
    let context = ComponentContext::new();
    let html = button.render(&props, &context)?;

    // Output HTML
    println!("{}", html.render());

    Ok(())
}
```

### 7. Project Structure

Recommended project structure:

```
my-ruitl-app/
├── Cargo.toml
├── build.rs
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── generated/          # Generated Rust files
│   │   ├── mod.rs
│   │   ├── button.rs
│   │   └── usercard.rs
│   └── models/
│       └── user.rs
├── templates/              # RUITL template files
│   ├── Button.ruitl
│   ├── UserCard.ruitl
│   └── Layout.ruitl
└── static/
    ├── style.css
    └── images/
```

### 8. Advanced Features

#### Custom Types

Define custom types for props:

```rust
// src/models/user.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Post {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub author: User,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

```ruitl
// templates/PostCard.ruitl
import "crate::models" { Post, User }
import "chrono" { DateTime, Utc }

component PostCard {
    props {
        post: Post,
        show_author: bool = true,
    }
}

ruitl PostCard(props: PostCardProps) {
    <article class="post-card">
        <header class="post-header">
            <h2 class="post-title">{props.post.title}</h2>
            if props.show_author {
                <div class="post-author">
                    @UserAvatar(user: props.post.author.clone())
                    <span>by {props.post.author.name}</span>
                </div>
            }
            <time class="post-date">
                {props.post.created_at.format("%B %d, %Y").to_string()}
            </time>
        </header>

        <div class="post-content">
            {props.post.content}
        </div>
    </article>
}
```

#### Error Handling

Handle errors gracefully in templates:

```ruitl
ruitl SafeComponent(props: SafeComponentProps) {
    <div class="safe-component">
        match props.result {
            Ok(data) => {
                <div class="success">
                    <p>Success: {data}</p>
                </div>
            }
            Err(error) => {
                <div class="error">
                    <p>Error: {error.to_string()}</p>
                </div>
            }
        }
    </div>
}
```

### 9. Best Practices

1. **Keep templates focused** - One component per template file
2. **Use descriptive prop names** - Make intent clear
3. **Provide sensible defaults** - Reduce required props where possible
4. **Validate props** - Use the validation system for complex requirements
5. **Compose components** - Build complex UIs from simple, reusable parts
6. **Handle edge cases** - Always consider empty states and error conditions

### 10. IDE Setup

For the best development experience:

1. **VS Code**: Install the RUITL extension for syntax highlighting
2. **Language Server**: Configure the RUITL language server for autocompletion
3. **File associations**: Associate `.ruitl` files with the RUITL language

### 11. Debugging

#### Common Issues

1. **Parse errors**: Check syntax for missing braces, quotes, or semicolons
2. **Type errors**: Ensure prop types match Rust types exactly
3. **Import errors**: Verify import paths and exported items
4. **Missing components**: Check that referenced components are defined

#### Debug Output

Enable verbose compilation:

```bash
RUITL_DEBUG=1 cargo build
```

### 12. Migration from Runtime Library

If migrating from RUITL's runtime library approach:

1. **Extract component logic** to separate `.ruitl` files
2. **Convert `html!` macros** to template syntax
3. **Update imports** to use generated components
4. **Test incrementally** - migrate one component at a time

## Examples

See the `examples/` directory for complete working examples:

- `examples/templates/Button.ruitl` - Basic button component
- `examples/templates/UserCard.ruitl` - Complex component with composition
- `examples/template_compiler_demo.rs` - Full compilation workflow demo

## Resources

- [RUITL Documentation](https://docs.rs/ruitl)
- [Template Syntax Reference](./TEMPLATE_SYNTAX.md)
- [Component API Reference](./COMPONENT_API.md)
- [Examples Repository](https://github.com/chrisolson/ruitl/tree/main/examples)
