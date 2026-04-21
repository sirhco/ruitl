# RUITL Examples

A gallery of `.ruitl` templates and example binaries. Start here if you're
learning the language or evaluating RUITL for a project.

## Directory layout

| Path | Role |
|---|---|
| [`demo_templates/`](demo_templates/) | **Compilable** `.ruitl` files used by `server_integration.rs`. Live siblings are regenerated on `cargo build`. |
| [`syntax_showcase/`](syntax_showcase/) | **Reference material.** Hand-written templates showing every syntax feature; NOT compiled (they reference notional user-defined types). |
| `*.rs` | Six runnable example binaries. See the [Example binaries](#example-binaries) section. |

The crate's own templates live in [`../templates/`](../templates/) — small,
feature-focused files exercised by the integration tests. Per-feature
snapshot fixtures live in [`../tests/fixtures/snapshots/`](../tests/fixtures/snapshots/).

## Where to look first

Pick your learning goal, open the file. Each entry starts tiny and grows.

| Learning goal | File |
|---|---|
| Simplest possible component | [`../templates/Hello.ruitl`](../templates/Hello.ruitl) |
| Props with defaults | [`../templates/Button.ruitl`](../templates/Button.ruitl) |
| `if` / `else` conditional rendering | [`../templates/SimpleIf.ruitl`](../templates/SimpleIf.ruitl), [`../tests/fixtures/snapshots/conditionals.ruitl`](../tests/fixtures/snapshots/conditionals.ruitl) |
| `for` loops over a `Vec<T>` | [`../tests/fixtures/snapshots/loops.ruitl`](../tests/fixtures/snapshots/loops.ruitl) |
| `match` expressions with multiple arms | [`../templates/AdvancedFeatures.ruitl`](../templates/AdvancedFeatures.ruitl), [`../tests/fixtures/snapshots/match_arms.ruitl`](../tests/fixtures/snapshots/match_arms.ruitl) |
| Composition: `@Child(prop = value)` | [`../tests/fixtures/snapshots/composition.ruitl`](../tests/fixtures/snapshots/composition.ruitl), [`../tests/fixtures/composition/UserList.ruitl`](../tests/fixtures/composition/UserList.ruitl) |
| Generics `<T: Bound>` | [`../tests/fixtures/snapshots/generics.ruitl`](../tests/fixtures/snapshots/generics.ruitl) |
| Optional props via `if let Some(...)` | [`syntax_showcase/UserCard.ruitl`](syntax_showcase/UserCard.ruitl), [`demo_templates/DemoButton.ruitl`](demo_templates/DemoButton.ruitl) |
| Real HTTP server using compiled components | [`server_integration.rs`](server_integration.rs) + [`demo_templates/`](demo_templates/) |
| Everything at once (cheat sheet) | [`syntax_showcase/UserCard.ruitl`](syntax_showcase/UserCard.ruitl) |

## Example binaries

Run any of these with `cargo run --example <name>`.

- **`hello_world`** — Minimal runtime usage. Instantiates `Hello` and prints its HTML. Good first run after `git clone`.
- **`basic_usage`** — Hand-builds an `Html` tree with the `HtmlElement` builder API. Shows the runtime surface without `.ruitl` at all.
- **`html_output_demo`** — Demonstrates `Html::render`, escaping rules, `Html::Raw`, fragments, and the `Display` impl.
- **`template_compiler_demo`** — Drives the parser and code generator directly. Useful if you're embedding RUITL as a library.
- **`advanced_features_demo`** — Instantiates the `AdvancedFeatures` component with a variety of prop combinations to exercise `if`/`for`/`match`.
- **`server_integration`** — Full HTTP server on port 3000. Routes:
  - `/` — Home (hand-written `Page` component)
  - `/users` — Users list rendered from `UserCard`s
  - `/about` — About page with pre-rendered HTML content
  - `/demo` — **Uses compiled `DemoButton` + `DemoUserCard`** from `demo_templates/`. Proves sibling-file integration end-to-end.
  - `/api/users` — JSON endpoint

## Feature coverage matrix

| Feature | Status | Seen in |
|---|---|---|
| Interpolation `{expr}` | ✓ | Nearly every fixture |
| Props + defaults | ✓ | `templates/Button.ruitl` |
| Optional props `foo: T?` | ✓ | `templates/UserCard.ruitl`, `demo_templates/DemoButton.ruitl` |
| Attribute interpolation `class={expr}` | ✓ | `templates/Button.ruitl` |
| Boolean conditional attrs `disabled?={expr}` | ✓ | `syntax_showcase/Button.ruitl` |
| `if` / `else` | ✓ | `templates/SimpleIf.ruitl` |
| `if let Some(x) = ...` | ✓ | `syntax_showcase/UserCard.ruitl` |
| `for x in xs` | ✓ | `templates/AdvancedFeatures.ruitl` |
| Tuple pattern `for (k, v) in map` | ✓ | parser tests only — no user-facing example yet |
| `match e { ... }` | ✓ | `templates/AdvancedFeatures.ruitl` |
| Composition `@Child(prop=val)` | ✓ | `tests/fixtures/composition/UserList.ruitl` |
| Generics `<T: Clone + Debug>` | ✓ | `tests/fixtures/snapshots/generics.ruitl` |
| Namespaced/hyphenated attrs (`aria-*`, `xmlns:xlink`) | ✓ | parser tests |
| Import statements | ✓ | `syntax_showcase/*.ruitl` |

## Known limitations / intentionally not shown

- **No children/slots feature yet.** A component cannot receive arbitrary child HTML (like React's `{children}` or templ's `@Template` blocks). Layout wrappers currently compose by inlining the full inner tree per caller.
- **Raw HTML from an expression has no short form.** If you need to drop already-safe HTML into a template, precompute a `String` and use `Html::raw(...)` at the Rust boundary — there is no `{!expr}` or similar inside `.ruitl` yet.
- **Forms, data tables, nav bars** are deliberately absent. They reduce to patterns already shown (elements + attributes + loops) and adding them would pad the cookbook without new pedagogical value. If you hit a concrete ergonomics gap building one, open an issue.
- **Lifetime generics** `<'a>` are rejected at parse time. Use owned types (`String`, `Vec<T>`) instead.

## Editor support

`.ruitl` files currently open as plain text in most editors. A tree-sitter
grammar is planned for v0.3 (syntax highlighting for Neovim / Helix / Zed);
a full LSP (diagnostics, completion) lands after that. Track progress in
the root [`README.md`](../README.md#status) status table.
