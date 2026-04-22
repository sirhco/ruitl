//! Integration test for the `ruitl::testing` helpers.
//!
//! Runs only when `--features testing` is enabled; otherwise the module is
//! absent from the public surface. This mirrors how downstream consumers
//! will wire it: as a feature in their `[dev-dependencies]`.

#![cfg(feature = "testing")]

use ruitl::assert_html_contains;
use ruitl::testing::prelude::*;
use ruitl::Result;

#[derive(Debug, Clone)]
struct GreetProps {
    name: String,
}

impl ComponentProps for GreetProps {}

#[derive(Debug)]
struct Greet;

impl Component for Greet {
    type Props = GreetProps;

    fn render(&self, props: &Self::Props, _ctx: &ComponentContext) -> Result<Html> {
        use ruitl::html::{h1, HtmlElement};
        Ok(Html::Element(HtmlElement::new("section").child(
            Html::Element(h1().text(&format!("Hello, {}!", props.name))),
        )))
    }
}

#[test]
fn harness_and_assertion_roundtrip() {
    let harness = ComponentTestHarness::new(Greet);
    let html = harness
        .render(&GreetProps {
            name: "World".into(),
        })
        .unwrap();
    HtmlAssertion::new(&html)
        .contains("Hello, World!")
        .element_count("h1", 1)
        .not_contains("Goodbye");
}

#[test]
fn macro_accepts_html_and_str() {
    let harness = ComponentTestHarness::new(Greet);
    let html = harness
        .render(&GreetProps {
            name: "Macro".into(),
        })
        .unwrap();
    assert_html_contains!(&html, "Hello, Macro!");

    let rendered = html.render();
    assert_html_contains!(rendered.as_str(), "<h1>");
}
