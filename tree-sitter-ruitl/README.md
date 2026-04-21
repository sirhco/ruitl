# tree-sitter-ruitl

[Tree-sitter](https://tree-sitter.github.io/) grammar for the
[RUITL](../README.md) template language. Ships the parser source
(`grammar.js`), highlight queries (`queries/highlights.scm`), and Rust
expression injections (`queries/injections.scm`) so `.ruitl` files light up
in any tree-sitter-aware editor.

## Status

**v0.1 — structural parsing + highlight queries.** Good enough for syntax
highlighting and structural navigation. Not yet published to the
`nvim-treesitter` registry or the Zed extension store.

Scope deliberately stops at structure. Rust expressions inside `{ ... }`
are captured as opaque `rust_expression` nodes and delegated to the Rust
highlighter via `injections.scm`; this grammar never re-implements Rust
parsing.

## Build

Requires `tree-sitter-cli` (Node, >= 0.22). With the CLI installed:

```bash
cd tree-sitter-ruitl
npm install         # pulls tree-sitter-cli
npm run build       # tree-sitter generate → src/parser.c
npm test            # runs the fixtures under test/corpus/
```

`tree-sitter generate` produces `src/parser.c` which the Node binding /
C binding link against. The generated file is not committed — run
`npm run build` after any `grammar.js` change.

## Editor wiring

### Neovim (`nvim-treesitter`)

Until the grammar ships in the upstream registry, use the parser source
from this repo directly:

```lua
require('nvim-treesitter.parsers').get_parser_configs().ruitl = {
  install_info = {
    url = "~/development/ruitl/tree-sitter-ruitl", -- or git URL
    files = { "src/parser.c" },
    branch = "main",
    generate_requires_npm = true,
    requires_generate_from_grammar = true,
  },
  filetype = "ruitl",
}
vim.filetype.add({ extension = { ruitl = "ruitl" } })
-- Then:  :TSInstall ruitl
```

Copy `queries/highlights.scm` and `queries/injections.scm` to
`~/.config/nvim/queries/ruitl/` (or pass them via the parser install).

### Helix

Add to `languages.toml`:

```toml
[[language]]
name = "ruitl"
scope = "source.ruitl"
file-types = ["ruitl"]
roots = []
comment-token = "//"
grammar = "ruitl"

[[grammar]]
name = "ruitl"
source = { path = "/Users/you/development/ruitl/tree-sitter-ruitl" }
```

Run `hx -g fetch && hx -g build` to generate + build the parser.

### Zed

Zed auto-discovers tree-sitter extensions via its registry. Publish path
TBD; for now clone this directory into `~/.config/zed/extensions/ruitl/`.

## What the grammar covers

- File-level declarations: `import`, `component`, `ruitl`
- Generic parameters with trait bounds: `<T: Clone + Debug>`
- Props with defaults (`= expr`) and optional markers (`?`)
- Template body:
  - HTML elements (open/close and self-closing) with hyphenated +
    namespaced attributes (`aria-hidden`, `xmlns:xlink`)
  - Attribute interpolation (`class={expr}`) and conditional attrs
    (`disabled?={cond}`)
  - Expression interpolation (`{expr}`) anywhere in a body
  - Component composition (`@Child(prop: value, ...)`)
  - Control flow: `if`/`else`, `for x in xs { ... }`,
    `for (k, v) in pairs { ... }`, `match scrutinee { pat => { ... } }`
  - `<!DOCTYPE ...>`
- Line comments (`//`) and block comments (`/* ... */`)
- String literals with `\n`, `\t`, `\\`, `\"`, `\u{…}` escapes

## Known limitations

- **Embedded `format!("...{}", x)` with literal `{`/`}` in the format string
  breaks expression-span parsing.** The grammar's Rust-expression regex
  stops at the first `{` — it cannot track balanced braces inside string
  literals without an external C scanner, which is out of scope for v0.1.
  Workaround: precompute the string outside the template and interpolate
  the result, or surround the expression with an extra `{{ ... }}` wrapper.
  Semantic parsing always goes through `ruitl_compiler` — this affects only
  editor highlighting.
- **Rust expressions with unbalanced `()` or nested `(foo(bar))` deeper
  than one level** may under-match. Same root cause; same workaround.

## What's intentionally out of scope

- **Rust expression parsing.** `{ foo.bar(baz) + 1 }` is an opaque
  `rust_expression` span. Editors get proper Rust highlighting via the
  injection query. Completion inside those spans requires rust-analyzer,
  not this grammar.
- **Type expression parsing.** Prop / param types (`Vec<Option<String>>`,
  `HashMap<K, V>`, etc.) are captured as opaque `type_expr` regex tokens.
  Full Rust type parsing is rust-analyzer's job.
- **Semantic validation.** Unused props, undeclared components, missing
  closing tags — the runtime parser in `ruitl_compiler/src/parser.rs`
  catches these and will be surfaced via the forthcoming `ruitl-lsp`.

## Contributing

- `grammar.js` is the source of truth.
- Add tests to `test/corpus/*.txt` for every rule change. `tree-sitter
  test` runs them against the generated parser.
- Keep queries editor-agnostic — follow `nvim-treesitter` capture names
  so Neovim, Helix, and Zed all work without remapping.
