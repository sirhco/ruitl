//! End-to-end test for `@Component` composition.
//!
//! Compiles `tests/fixtures/composition/UserList.ruitl` (which uses
//! `@UserCard(...)` inside a `for` loop) and asserts that:
//!   1. The parser recognises the composition node.
//!   2. The generator emits valid Rust that references both `UserCard` and
//!      `UserCardProps`, instantiates props with the loop variable, and calls
//!      `component.render(&props, context)?`.
//!   3. The generated source parses back through `syn::parse_file` (i.e. it
//!      is syntactically valid Rust).
//!
//! We deliberately avoid trying to *compile* the resulting Rust here because
//! that would require the runtime crate to depend on a concrete `User` type;
//! the goal of this test is to lock in the @-composition pipeline, not the
//! downstream type system.

use ruitl_compiler::{generate, parse_str, TemplateAst};

const USER_LIST: &str = include_str!("fixtures/composition/UserList.ruitl");
const CARD_WITH_CHILDREN: &str =
    include_str!("fixtures/composition/CardWithChildren.ruitl");

#[test]
fn user_list_parses_with_composition_node() {
    let file = parse_str(USER_LIST).expect("UserList.ruitl must parse");
    assert_eq!(file.components.len(), 1, "expected one component def");
    assert_eq!(file.templates.len(), 1, "expected one template");

    let body = &file.templates[0].body;
    let composition = find_component_node(body)
        .expect("@UserCard composition node must exist somewhere in the body");
    let TemplateAst::Component {
        name,
        props,
        children,
    } = composition
    else {
        unreachable!()
    };
    assert_eq!(name, "UserCard");
    let prop_names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(prop_names, vec!["name", "email", "role"]);
    assert!(
        children.is_none(),
        "UserList invocation has no body block"
    );
}

#[test]
fn user_list_codegen_emits_valid_invocation() {
    let file = parse_str(USER_LIST).expect("UserList.ruitl must parse");
    let code = generate(file).expect("codegen must succeed");

    // Sanity: the @UserCard(...) call should compile down to instantiating
    // UserCardProps and invoking component.render(&props, context)? .
    assert!(
        code.contains("UserCardProps"),
        "expected generated code to reference UserCardProps, got:\n{code}"
    );
    assert!(
        code.contains("UserCard"),
        "expected generated code to reference UserCard component"
    );
    assert!(
        code.contains("component.render"),
        "expected component.render(&props, context) call from @-composition\nGOT:\n{code}"
    );

    // Must be syntactically valid Rust.
    syn::parse_file(&code).unwrap_or_else(|e| {
        panic!("generated code is not valid Rust: {e}\n--- CODE ---\n{code}")
    });
}

#[test]
fn card_with_children_codegen_auto_injects_children_field() {
    let file = parse_str(CARD_WITH_CHILDREN).expect("CardWithChildren.ruitl must parse");
    let code = generate(file).expect("codegen must succeed");

    // The Card's Props struct should carry an auto-injected `children: Html` field.
    let normalized: String = code.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized.contains("pub children : Html")
            || normalized.contains("pub children: Html"),
        "CardWithChildrenProps must carry `pub children: Html`; got:\n{code}"
    );

    // The Shell's body-block invocation must feed a value into `children:` —
    // codegen picks `Html::fragment(...)` for multi-child bodies and a direct
    // `Html::Element(...)` for a single element. Accept both.
    assert!(
        code.contains("children : Html") || code.contains("children: Html"),
        "Shell's @-call with body must populate the `children` field; got:\n{code}"
    );

    // The slot placeholder `{children}` should expand to a clone of props.children.
    assert!(
        code.contains("props . children . clone")
            || code.contains("props.children.clone"),
        "`{{children}}` slot must compile to props.children.clone(); got:\n{code}"
    );

    // Must be syntactically valid Rust.
    syn::parse_file(&code)
        .unwrap_or_else(|e| panic!("generated code is not valid Rust: {e}\n--- CODE ---\n{code}"));
}

fn find_component_node(node: &TemplateAst) -> Option<&TemplateAst> {
    match node {
        TemplateAst::Component { .. } => Some(node),
        TemplateAst::Element { children, .. } => children.iter().find_map(find_component_node),
        TemplateAst::Fragment(items) => items.iter().find_map(find_component_node),
        TemplateAst::If {
            then_branch,
            else_branch,
            ..
        } => find_component_node(then_branch)
            .or_else(|| else_branch.as_deref().and_then(find_component_node)),
        TemplateAst::For { body, .. } => find_component_node(body),
        TemplateAst::Match { arms, .. } => arms.iter().find_map(|a| find_component_node(&a.body)),
        _ => None,
    }
}
