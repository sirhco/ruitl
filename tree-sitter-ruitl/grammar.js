/**
 * @file Tree-sitter grammar for RUITL (Rust UI Template Language)
 * @author chrisolson + contributors
 * @license MIT
 *
 * Scope: syntax highlighting and structural navigation for .ruitl files.
 * Rust expressions embedded inside `{ ... }` are intentionally captured as
 * opaque `rust_expression` nodes — a real Rust parse requires rust-analyzer
 * and is out of scope for a tree-sitter grammar.
 *
 * See the runtime parser at `ruitl_compiler/src/parser.rs` for the
 * reference grammar. Where the hand-written parser is more permissive than
 * this grammar (e.g. accepting trailing commas), we follow the hand-written
 * parser's behavior so files that RUITL compiles also parse cleanly in
 * tree-sitter.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: 'ruitl',

  extras: $ => [
    /\s+/,
    $.line_comment,
    $.block_comment,
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat(choice(
      $.import_declaration,
      $.component_declaration,
      $.ruitl_declaration,
    )),

    // -----------------------------------------------------------------
    // Imports: `import "path" { A, B, C }`
    // -----------------------------------------------------------------
    import_declaration: $ => seq(
      'import',
      field('path', $.string_literal),
      '{',
      optional(seq(
        $.identifier,
        repeat(seq(',', $.identifier)),
        optional(','),
      )),
      '}',
    ),

    // -----------------------------------------------------------------
    // component Name<T: Bound + Bound> { props { field: Type = default, ... } }
    // -----------------------------------------------------------------
    component_declaration: $ => seq(
      'component',
      field('name', $.identifier),
      optional(field('generics', $.generic_params)),
      '{',
      optional($.props_block),
      '}',
    ),

    props_block: $ => seq(
      'props',
      '{',
      repeat($.prop_def),
      '}',
    ),

    prop_def: $ => seq(
      field('name', $.identifier),
      ':',
      field('type', $.type_expr),
      optional(choice(
        seq('=', field('default', alias($._expr_csv, $.rust_expression))),
        field('optional', $.optional_marker),
      )),
      optional(','),
    ),

    // Sentinel node for `?` on a prop (`foo: Type?`). Modelled as a named
    // node so it appears in the AST under the `optional` field; anonymous
    // literal tokens are stripped from the tree by tree-sitter.
    optional_marker: _ => '?',

    // -----------------------------------------------------------------
    // Generics: `<T, U: Clone + Debug>`
    // -----------------------------------------------------------------
    generic_params: $ => seq(
      '<',
      $.generic_param,
      repeat(seq(',', $.generic_param)),
      optional(','),
      '>',
    ),

    generic_param: $ => seq(
      field('name', $.identifier),
      optional(seq(
        ':',
        $.type_bound,
        repeat(seq('+', $.type_bound)),
      )),
    ),

    type_bound: $ => $.type_expr,

    // -----------------------------------------------------------------
    // ruitl Name<T>(param: Type, ...) { TEMPLATE BODY }
    // -----------------------------------------------------------------
    ruitl_declaration: $ => seq(
      'ruitl',
      field('name', $.identifier),
      optional(field('generics', $.generic_params)),
      '(',
      optional(seq(
        $.param_def,
        repeat(seq(',', $.param_def)),
        optional(','),
      )),
      ')',
      '{',
      repeat($._template_node),
      '}',
    ),

    param_def: $ => seq(
      field('name', $.identifier),
      ':',
      field('type', $.type_expr),
    ),

    // -----------------------------------------------------------------
    // Template body nodes
    // -----------------------------------------------------------------
    _template_node: $ => choice(
      $.element,
      $.self_closing_element,
      $.doctype,
      $.expression_node,
      $.component_invocation,
      $.if_statement,
      $.for_statement,
      $.match_statement,
      $.text,
      // A bare word inside a template body is text. Aliased from
      // `identifier` so tree-sitter's keyword extraction (`word:
      // $.identifier`) still surfaces `if`/`else`/`for`/`in`/`match` as
      // control-flow keywords when they appear alone — only non-keyword
      // words fall through to this alias and render as text.
      alias($.identifier, $.text),
    ),

    doctype: _ => token(seq(
      '<!DOCTYPE',
      /[^>]*/,
      '>',
    )),

    element: $ => seq(
      '<',
      field('tag', $.tag_name),
      repeat($.attribute),
      '>',
      repeat($._template_node),
      '</',
      field('closing_tag', $.tag_name),
      '>',
    ),

    self_closing_element: $ => seq(
      '<',
      field('tag', $.tag_name),
      repeat($.attribute),
      '/>',
    ),

    tag_name: _ => /[A-Za-z][A-Za-z0-9_\-]*/,

    attribute: $ => seq(
      field('name', $.attribute_name),
      optional(field('conditional', '?')),
      optional(seq(
        '=',
        field('value', choice($.string_literal, $.attribute_expression)),
      )),
    ),

    // HTML attribute names allow `-` and `:` in addition to identifier chars.
    attribute_name: _ => /[A-Za-z_][A-Za-z0-9_:\-]*/,

    // `class={expr}` / `disabled?={cond}` — the braced expression is an
    // arbitrary Rust expression.
    attribute_expression: $ => seq(
      '{',
      field('expr', $.rust_expression),
      '}',
    ),

    // `{expr}` standalone interpolation inside a template body.
    expression_node: $ => seq(
      '{',
      field('expr', $.rust_expression),
      '}',
    ),

    // `@Component(prop: value, prop: value)`
    component_invocation: $ => seq(
      '@',
      field('name', $.identifier),
      '(',
      optional(seq(
        $.component_prop,
        repeat(seq(',', $.component_prop)),
        optional(','),
      )),
      ')',
    ),

    component_prop: $ => seq(
      field('name', $.identifier),
      ':',
      field('value', alias($._expr_csv, $.rust_expression)),
    ),

    // -----------------------------------------------------------------
    // Control flow inside template bodies
    // -----------------------------------------------------------------
    if_statement: $ => seq(
      'if',
      field('condition', alias($._expr_stmt, $.rust_expression)),
      '{',
      repeat($._template_node),
      '}',
      optional(seq(
        'else',
        choice(
          $.if_statement,
          seq('{', repeat($._template_node), '}'),
        ),
      )),
    ),

    for_statement: $ => seq(
      'for',
      field('binding', choice($.identifier, $.tuple_pattern)),
      'in',
      field('iterable', alias($._expr_stmt, $.rust_expression)),
      '{',
      repeat($._template_node),
      '}',
    ),

    tuple_pattern: $ => seq(
      '(',
      $.identifier,
      repeat(seq(',', $.identifier)),
      optional(','),
      ')',
    ),

    match_statement: $ => seq(
      'match',
      field('scrutinee', alias($._expr_stmt, $.rust_expression)),
      '{',
      repeat($.match_arm),
      '}',
    ),

    match_arm: $ => seq(
      field('pattern', alias($._expr_arm, $.rust_expression)),
      '=>',
      '{',
      repeat($._template_node),
      '}',
      optional(','),
    ),

    // -----------------------------------------------------------------
    // Rust expression spans. A real Rust parse is out of scope — every
    // context here uses a regex tuned to stop at its delimiter:
    //
    //   rust_expression  — inside `{ ... }` interpolations. Stops at `{`/`}`/`(`/`)`.
    //                      Permits single-level `(...)` so `foo(x, y)` works.
    //   _expr_stmt       — inside `if <e> {`, `for x in <e> {`, `match <e> {`.
    //                      Stops at `{`/`}`.
    //   _expr_csv        — inside `name: <e>,` (prop default or component prop).
    //                      Stops at `,`, `)`, `{`, `}`. Permits single-level `(...)`.
    //   _expr_arm        — match arm pattern before `=>`. Stops at `=>` / `{` / `}`.
    //
    // Each is a token to avoid interleaving with whitespace-`extras`.
    // -----------------------------------------------------------------
    rust_expression: _ => token(prec(-1, /[^\{\}\(\)]+(?:\([^\)]*\)[^\{\}\(\)]*)*/)),

    _expr_stmt: _ => token(prec(-1, /[^\{\}]+/)),
    _expr_csv: _ => token(prec(-1, /[^,\)\{\}]+(?:\([^\)]*\)[^,\)\{\}]*)*/)),
    // Match arm pattern: stop at `=>`, `{`, `}`. `=` without a following `>`
    // would need lookahead (unsupported by the regex engine), so the pattern
    // `a == b` in an arm is accepted by stopping only at `=>` — the `=>`
    // separator uses two characters so `==` matches fine.
    _expr_arm: _ => token(prec(-1, /[^=\{\}]+/)),

    // -----------------------------------------------------------------
    // Type expressions in prop / param declarations. Structural rather
    // than regex-based so it can't accidentally swallow terminators like
    // `?`, `,`, `)`, `>`. Covers identifiers, path segments, generics,
    // references, tuples, and arrays — enough for real-world prop types.
    // Anything more exotic is rust-analyzer's problem.
    // -----------------------------------------------------------------
    type_expr: $ => choice(
      $._type_path,
      $._type_reference,
      $._type_tuple,
      $._type_array,
    ),

    _type_path: $ => seq(
      $.identifier,
      repeat(seq('::', $.identifier)),
      optional(seq(
        '<',
        $.type_expr,
        repeat(seq(',', $.type_expr)),
        optional(','),
        '>',
      )),
    ),

    _type_reference: $ => seq('&', optional('mut'), $.type_expr),

    _type_tuple: $ => seq(
      '(',
      optional(seq(
        $.type_expr,
        repeat(seq(',', $.type_expr)),
        optional(','),
      )),
      ')',
    ),

    _type_array: $ => seq('[', $.type_expr, optional(seq(';', $.rust_expression)), ']'),

    // -----------------------------------------------------------------
    // Literals and comments
    // -----------------------------------------------------------------
    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,

    string_literal: $ => seq(
      '"',
      repeat(choice(
        $.escape_sequence,
        /[^"\\]/,
      )),
      '"',
    ),

    escape_sequence: _ => /\\(?:[nrt\\"'{0}]|x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\})/,

    // Text inside a template body covers ONLY non-word sequences —
    // punctuation, digits, symbols, Unicode glyphs, etc. Plain words are
    // captured through `alias($.identifier, $.text)` in `_template_node`
    // so tree-sitter's keyword extraction (via `word: $.identifier`) can
    // still surface `if`/`else`/`for`/`in`/`match` when they appear alone.
    //
    // The regex stops at any whitespace — whitespace between words is
    // consumed by `extras`. Rendering consumers can re-concatenate from
    // node source ranges if original whitespace matters.
    text: _ => token(/[^\sA-Za-z_<{}@][^\s<{@]*/),

    line_comment: _ => token(seq('//', /[^\n]*/)),

    block_comment: _ => token(seq('/*', /(?:[^*]|\*[^/])*/, '*/')),
  },

  // No explicit conflicts — tree-sitter's LR(1) analysis handles the
  // `{expr}` / `{` template-body ambiguity via lookahead without a declared
  // conflict. If a future rule re-introduces one, list it here.
  conflicts: _ => [],
});
