//! End-to-end tests for "did you mean" codegen diagnostics.
//!
//! Each fixture in `tests/fixtures/errors/` is intentionally broken: we
//! assert that codegen refuses it and that the error message carries a
//! matching suggestion line.

use ruitl_compiler::{generate, parse_str};

#[test]
fn unknown_component_suggests_declared_name() {
    let src = include_str!("fixtures/errors/typo_component.ruitl");
    let ast = parse_str(src).expect("fixture must parse — only codegen is broken");
    let err = generate(ast).expect_err("codegen must reject @Buttom");
    let msg = err.to_string();
    assert!(
        msg.contains("Unknown component `Buttom`"),
        "message should name the unknown component, got:\n{msg}"
    );
    assert!(
        msg.contains("did you mean `Button`?"),
        "message should suggest `Button`, got:\n{msg}"
    );
}

#[test]
fn unknown_prop_suggests_declared_prop() {
    let src = include_str!("fixtures/errors/typo_prop.ruitl");
    let ast = parse_str(src).expect("fixture must parse");
    let err = generate(ast).expect_err("codegen must reject @Button(texx: ...)");
    let msg = err.to_string();
    assert!(
        msg.contains("No prop `texx` on `ButtonProps`"),
        "message should name the unknown prop, got:\n{msg}"
    );
    assert!(
        msg.contains("did you mean `text`?"),
        "message should suggest `text`, got:\n{msg}"
    );
}
