; Inject the `rust` language into every `rust_expression` node so editors
; that support tree-sitter injections (Neovim, Helix) run the Rust
; highlighter inside `{ ... }` spans. Without injection, Rust expressions
; render as opaque text.

((rust_expression) @injection.content
 (#set! injection.language "rust")
 (#set! injection.include-children))
