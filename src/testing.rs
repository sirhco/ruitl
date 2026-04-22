//! Component testing utilities.
//!
//! Gated behind the `testing` feature (or `cfg(test)` inside this crate).
//! Consumers pull these into their test modules via:
//!
//! ```toml
//! [dev-dependencies]
//! ruitl = { version = "0.1", features = ["testing"] }
//! ```
//!
//! The harness centralises the boilerplate of "make a context, invoke
//! `render`, unwrap, inspect", so component tests read as:
//!
//! ```ignore
//! use ruitl::testing::prelude::*;
//!
//! let html = ComponentTestHarness::new(Button)
//!     .render(&ButtonProps { text: "Go".into(), variant: "primary".into() })
//!     .unwrap();
//! assert_html_contains!(&html, "Go");
//! assert_html_contains!(&html, "btn-primary");
//! ```
//!
//! The scope is deliberately small: imperative inspection helpers, not a
//! replacement for `insta`-backed snapshots. Use both.

use crate::component::{Component, ComponentContext};
use crate::error::Result;
use crate::html::Html;

/// A thin wrapper around a `Component` that manages `ComponentContext`
/// creation and error propagation for tests.
pub struct ComponentTestHarness<C: Component> {
    component: C,
    context: ComponentContext,
}

impl<C: Component> ComponentTestHarness<C> {
    /// Create a harness with a default (empty) context.
    pub fn new(component: C) -> Self {
        Self {
            component,
            context: ComponentContext::new(),
        }
    }

    /// Override the context (useful when a component reads request path,
    /// query params, or custom data from it).
    pub fn with_context(mut self, context: ComponentContext) -> Self {
        self.context = context;
        self
    }

    /// Render the component and return the raw `Html` tree.
    pub fn render(&self, props: &C::Props) -> Result<Html> {
        self.component.render(props, &self.context)
    }

    /// Render and flatten to a `String` — most tests want this.
    pub fn render_string(&self, props: &C::Props) -> Result<String> {
        Ok(self.render(props)?.render())
    }
}

/// Assertion helpers wrapping a rendered `Html` tree. Substring / count
/// checks stay short and readable in test bodies.
pub struct HtmlAssertion {
    rendered: String,
}

impl HtmlAssertion {
    /// Build from a rendered `Html`. Renders to string once; subsequent
    /// assertions are O(n) substring scans against the cached output.
    pub fn new(html: &Html) -> Self {
        Self {
            rendered: html.render(),
        }
    }

    /// Build directly from a pre-rendered string.
    pub fn from_string<S: Into<String>>(rendered: S) -> Self {
        Self {
            rendered: rendered.into(),
        }
    }

    /// Assert the rendered output contains `needle` (returns `self` for
    /// chained assertions).
    pub fn contains(self, needle: &str) -> Self {
        assert!(
            self.rendered.contains(needle),
            "expected rendered HTML to contain `{needle}`, got:\n{}",
            self.rendered
        );
        self
    }

    /// Assert the rendered output does not contain `needle`.
    pub fn not_contains(self, needle: &str) -> Self {
        assert!(
            !self.rendered.contains(needle),
            "expected rendered HTML NOT to contain `{needle}`, got:\n{}",
            self.rendered
        );
        self
    }

    /// Assert the rendered output contains exactly `n` occurrences of an
    /// element's opening tag like `<li` (matches `<li>` and `<li class=...>`).
    pub fn element_count(self, tag: &str, n: usize) -> Self {
        let needle = format!("<{tag}");
        let count = self.rendered.matches(&needle).count();
        assert_eq!(
            count, n,
            "expected {n} occurrence(s) of `<{tag}`, got {count} in:\n{}",
            self.rendered
        );
        self
    }

    /// Access the raw rendered string for custom checks.
    pub fn as_str(&self) -> &str {
        &self.rendered
    }
}

/// Convenience: assert a rendered `Html` (or `&String`/`&str`) contains a
/// substring. Works on anything that derefs to `str` via `.to_string()` or
/// `Html::render()` — whichever the caller hands in.
#[macro_export]
macro_rules! assert_html_contains {
    ($html:expr, $needle:expr $(,)?) => {{
        let rendered: String = $crate::testing::__render_for_assert($html);
        assert!(
            rendered.contains($needle),
            "expected rendered HTML to contain `{}`, got:\n{}",
            $needle,
            rendered
        );
    }};
}

/// Convenience: assert a rendered `Html` (or `&str`) exactly matches
/// `expected`. Mostly useful for trivial leaf components; larger diffs
/// belong in `insta` snapshots.
#[macro_export]
macro_rules! assert_renders_to {
    ($html:expr, $expected:expr $(,)?) => {{
        let rendered: String = $crate::testing::__render_for_assert($html);
        assert_eq!(rendered, $expected);
    }};
}

/// Internal helper for the assertion macros. Accepts either a `&Html` (via
/// `.render()`) or anything that deref-converts to `&str`.
#[doc(hidden)]
pub fn __render_for_assert<T: Renderable>(value: T) -> String {
    value.render_to_string()
}

/// Object-safe-ish adapter the macros dispatch over. Two blanket impls
/// cover the common cases: `&Html` renders via `.render()`, and any `AsRef<str>`
/// (which covers `&str`, `&String`) passes its string through as-is.
pub trait Renderable {
    fn render_to_string(self) -> String;
}

impl Renderable for &Html {
    fn render_to_string(self) -> String {
        self.render()
    }
}

impl Renderable for &str {
    fn render_to_string(self) -> String {
        self.to_string()
    }
}

impl Renderable for &String {
    fn render_to_string(self) -> String {
        self.clone()
    }
}

impl Renderable for String {
    fn render_to_string(self) -> String {
        self
    }
}

/// Grab-bag re-export so test bodies can `use ruitl::testing::prelude::*;`
/// and not think about individual symbol paths.
pub mod prelude {
    pub use super::{ComponentTestHarness, HtmlAssertion};
    pub use crate::component::{Component, ComponentContext, ComponentProps};
    pub use crate::html::Html;
    // Macros from this crate are accessible via the root of the caller's
    // crate graph, not this module, so they're not re-exported here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::ComponentProps;
    use crate::html::{div, HtmlElement};

    #[derive(Debug, Clone)]
    struct EchoProps {
        msg: String,
    }

    impl ComponentProps for EchoProps {}

    #[derive(Debug)]
    struct Echo;

    impl Component for Echo {
        type Props = EchoProps;

        fn render(
            &self,
            props: &Self::Props,
            _ctx: &ComponentContext,
        ) -> Result<Html> {
            Ok(Html::Element(div().class("echo").text(&props.msg)))
        }
    }

    #[test]
    fn harness_renders_component() {
        let harness = ComponentTestHarness::new(Echo);
        let html = harness
            .render(&EchoProps {
                msg: "hello".into(),
            })
            .unwrap();
        HtmlAssertion::new(&html)
            .contains("hello")
            .contains("class=\"echo\"");
    }

    #[test]
    fn harness_render_string_matches_render() {
        let harness = ComponentTestHarness::new(Echo);
        let s = harness
            .render_string(&EchoProps {
                msg: "alpha".into(),
            })
            .unwrap();
        assert!(s.contains("alpha"));
    }

    #[test]
    fn assertion_element_count() {
        let tree = Html::Element(
            HtmlElement::new("ul")
                .child(Html::Element(HtmlElement::new("li").text("a")))
                .child(Html::Element(HtmlElement::new("li").text("b")))
                .child(Html::Element(HtmlElement::new("li").text("c"))),
        );
        HtmlAssertion::new(&tree).element_count("li", 3);
    }

    #[test]
    #[should_panic(expected = "expected rendered HTML to contain `nope`")]
    fn assertion_contains_panics_on_miss() {
        let html = Html::Element(div().text("yep"));
        HtmlAssertion::new(&html).contains("nope");
    }
}
