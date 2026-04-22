; Tree-sitter highlight captures for .ruitl files.
;
; Capture naming follows the `nvim-treesitter` convention so the queries
; drop into any tree-sitter-aware editor (Neovim, Helix, Zed) without
; remapping. Capture groups:
;   @keyword      — `component`, `ruitl`, `props`, `import`, `if`, `else`, `for`, `in`, `match`
;   @type         — type expressions and generic parameter names
;   @attribute    — HTML attribute names
;   @tag          — HTML element tag names
;   @constructor  — component names (in declarations and invocations)
;   @property     — prop / param identifiers
;   @variable     — for-loop bindings
;   @string       — string literals
;   @punctuation.bracket / @punctuation.delimiter — structural marks
;   @comment      — // and /* ... */

; -------------------------------------------------------------------------
; Keywords
; -------------------------------------------------------------------------
"component" @keyword
"ruitl" @keyword
"props" @keyword
"import" @keyword
"if" @keyword.conditional
"else" @keyword.conditional
"for" @keyword.repeat
"in" @keyword.repeat
"match" @keyword.conditional

; -------------------------------------------------------------------------
; Declarations
; -------------------------------------------------------------------------
(component_declaration name: (identifier) @constructor)
(ruitl_declaration name: (identifier) @constructor)

(generic_param name: (identifier) @type.parameter)
(type_bound (type_expr) @type)

(prop_def
  name: (identifier) @property
  type: (type_expr) @type)

(param_def
  name: (identifier) @property
  type: (type_expr) @type)

; -------------------------------------------------------------------------
; Template body
; -------------------------------------------------------------------------
(element tag: (tag_name) @tag)
(element closing_tag: (tag_name) @tag)
(self_closing_element tag: (tag_name) @tag)

(attribute name: (attribute_name) @attribute)
(attribute conditional: "?" @operator)

(component_invocation name: (identifier) @constructor)
(component_prop name: (identifier) @property)

(for_statement binding: (identifier) @variable)
(for_statement binding: (tuple_pattern (identifier) @variable))

; -------------------------------------------------------------------------
; Expressions and literals
; -------------------------------------------------------------------------
(rust_expression) @embedded
(string_literal) @string
(escape_sequence) @string.escape
(doctype) @tag.doctype
(text) @none

; -------------------------------------------------------------------------
; Comments
; -------------------------------------------------------------------------
(line_comment) @comment
(block_comment) @comment

; -------------------------------------------------------------------------
; Punctuation (optional but common in nvim-treesitter highlighting)
; -------------------------------------------------------------------------
"{" @punctuation.bracket
"}" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"<" @punctuation.bracket
">" @punctuation.bracket
"</" @punctuation.bracket
"/>" @punctuation.bracket
"," @punctuation.delimiter
":" @punctuation.delimiter
"=>" @punctuation.delimiter
"=" @operator
"+" @operator
"@" @punctuation.special
