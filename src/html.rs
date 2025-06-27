//! HTML rendering and manipulation utilities

use crate::error::{Result, RuitlError};
use html_escape::{encode_quoted_attribute, encode_text};
use std::collections::HashMap;
use std::fmt::{self, Display, Write};

/// Represents an HTML element with attributes and children
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlElement {
    pub tag: String,
    pub attributes: HashMap<String, HtmlAttribute>,
    pub children: Vec<Html>,
    pub self_closing: bool,
}

/// Represents an HTML attribute with optional value
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlAttribute {
    /// Attribute with a value (e.g., class="example")
    Value(String),
    /// Boolean attribute (e.g., disabled)
    Boolean,
    /// Multiple values (e.g., class="one two three")
    List(Vec<String>),
}

/// Main HTML content type that can be rendered
#[derive(Debug, Clone, PartialEq)]
pub enum Html {
    /// Text content (will be escaped)
    Text(String),
    /// Raw HTML content (will not be escaped)
    Raw(String),
    /// HTML element
    Element(HtmlElement),
    /// Fragment containing multiple HTML nodes
    Fragment(Vec<Html>),
    /// Empty/void content
    Empty,
}

impl HtmlElement {
    /// Create a new HTML element
    pub fn new<S: Into<String>>(tag: S) -> Self {
        Self {
            tag: tag.into(),
            attributes: HashMap::new(),
            children: Vec::new(),
            self_closing: false,
        }
    }

    /// Create a self-closing element (like <img>, <br>, etc.)
    pub fn self_closing<S: Into<String>>(tag: S) -> Self {
        Self {
            tag: tag.into(),
            attributes: HashMap::new(),
            children: Vec::new(),
            self_closing: true,
        }
    }

    /// Add an attribute with a value
    pub fn attr<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.attributes
            .insert(key.into(), HtmlAttribute::Value(value.into()));
        self
    }

    /// Add a boolean attribute
    pub fn bool_attr<K: Into<String>>(mut self, key: K) -> Self {
        self.attributes.insert(key.into(), HtmlAttribute::Boolean);
        self
    }

    /// Add a class attribute
    pub fn class<S: Into<String>>(mut self, class: S) -> Self {
        let class_name = class.into();
        match self.attributes.get_mut("class") {
            Some(HtmlAttribute::Value(existing)) => {
                *existing = format!("{} {}", existing, class_name);
            }
            Some(HtmlAttribute::List(list)) => {
                list.push(class_name);
            }
            _ => {
                self.attributes
                    .insert("class".to_string(), HtmlAttribute::Value(class_name));
            }
        }
        self
    }

    /// Add multiple classes
    pub fn classes<I, S>(mut self, classes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let class_list: Vec<String> = classes.into_iter().map(|s| s.into()).collect();
        if !class_list.is_empty() {
            self.attributes
                .insert("class".to_string(), HtmlAttribute::List(class_list));
        }
        self
    }

    /// Add an ID attribute
    pub fn id<S: Into<String>>(mut self, id: S) -> Self {
        self.attributes
            .insert("id".to_string(), HtmlAttribute::Value(id.into()));
        self
    }

    /// Add a child element
    pub fn child(mut self, child: Html) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn children<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = Html>,
    {
        self.children.extend(children);
        self
    }

    /// Add text content as a child
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.children.push(Html::Text(text.into()));
        self
    }

    /// Add raw HTML content as a child
    pub fn raw<S: Into<String>>(mut self, html: S) -> Self {
        self.children.push(Html::Raw(html.into()));
        self
    }

    /// Check if this element has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Check if this element is self-closing
    pub fn is_self_closing(&self) -> bool {
        self.self_closing || is_void_element(&self.tag)
    }
}

impl HtmlAttribute {
    /// Render the attribute as a string
    pub fn render(&self) -> String {
        match self {
            HtmlAttribute::Value(value) => format!("\"{}\"", encode_quoted_attribute(value)),
            HtmlAttribute::Boolean => String::new(),
            HtmlAttribute::List(list) => {
                let joined = list.join(" ");
                format!("\"{}\"", encode_quoted_attribute(&joined))
            }
        }
    }

    /// Check if this is a boolean attribute
    pub fn is_boolean(&self) -> bool {
        matches!(self, HtmlAttribute::Boolean)
    }
}

impl Html {
    /// Create text content
    pub fn text<S: Into<String>>(content: S) -> Self {
        Html::Text(content.into())
    }

    /// Create raw HTML content
    pub fn raw<S: Into<String>>(content: S) -> Self {
        Html::Raw(content.into())
    }

    /// Create an element
    pub fn element<S: Into<String>>(tag: S) -> HtmlElement {
        HtmlElement::new(tag)
    }

    /// Create a fragment
    pub fn fragment<I>(children: I) -> Self
    where
        I: IntoIterator<Item = Html>,
    {
        Html::Fragment(children.into_iter().collect())
    }

    /// Create empty content
    pub fn empty() -> Self {
        Html::Empty
    }

    /// Render the HTML to a string
    pub fn render(&self) -> String {
        let mut output = String::new();
        self.render_to(&mut output).unwrap_or_default();
        output
    }

    /// Render the HTML to a writer
    pub fn render_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Html::Text(text) => {
                write!(writer, "{}", encode_text(text))
                    .map_err(|e| RuitlError::render(format!("Failed to write text: {}", e)))?;
            }
            Html::Raw(html) => {
                write!(writer, "{}", html)
                    .map_err(|e| RuitlError::render(format!("Failed to write raw HTML: {}", e)))?;
            }
            Html::Element(element) => {
                element.render_to(writer)?;
            }
            Html::Fragment(children) => {
                for child in children {
                    child.render_to(writer)?;
                }
            }
            Html::Empty => {
                // Nothing to render
            }
        }
        Ok(())
    }

    /// Check if this HTML is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Html::Empty => true,
            Html::Text(text) => text.is_empty(),
            Html::Raw(html) => html.is_empty(),
            Html::Fragment(children) => {
                children.is_empty() || children.iter().all(|c| c.is_empty())
            }
            Html::Element(element) => false, // Elements are never considered empty
        }
    }

    /// Get the text content (without HTML tags)
    pub fn text_content(&self) -> String {
        match self {
            Html::Text(text) => text.clone(),
            Html::Raw(_) => String::new(), // Raw HTML doesn't contribute to text content
            Html::Element(element) => element
                .children
                .iter()
                .map(|c| c.text_content())
                .collect::<Vec<_>>()
                .join(""),
            Html::Fragment(children) => children
                .iter()
                .map(|c| c.text_content())
                .collect::<Vec<_>>()
                .join(""),
            Html::Empty => String::new(),
        }
    }
}

impl HtmlElement {
    /// Render the element to a writer
    pub fn render_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Opening tag
        write!(writer, "<{}", self.tag)
            .map_err(|e| RuitlError::render(format!("Failed to write opening tag: {}", e)))?;

        // Attributes
        for (key, value) in &self.attributes {
            match value {
                HtmlAttribute::Boolean => {
                    write!(writer, " {}", key).map_err(|e| {
                        RuitlError::render(format!("Failed to write boolean attribute: {}", e))
                    })?;
                }
                _ => {
                    write!(writer, " {}={}", key, value.render()).map_err(|e| {
                        RuitlError::render(format!("Failed to write attribute: {}", e))
                    })?;
                }
            }
        }

        if self.is_self_closing() {
            write!(writer, " />").map_err(|e| {
                RuitlError::render(format!("Failed to write self-closing tag: {}", e))
            })?;
        } else {
            write!(writer, ">")
                .map_err(|e| RuitlError::render(format!("Failed to write tag close: {}", e)))?;

            // Children
            for child in &self.children {
                child.render_to(writer)?;
            }

            // Closing tag
            write!(writer, "</{}>", self.tag)
                .map_err(|e| RuitlError::render(format!("Failed to write closing tag: {}", e)))?;
        }

        Ok(())
    }
}

impl Display for Html {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl Display for HtmlElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Html::Element(self.clone()).render())
    }
}

impl From<HtmlElement> for Html {
    fn from(element: HtmlElement) -> Self {
        Html::Element(element)
    }
}

impl From<String> for Html {
    fn from(text: String) -> Self {
        Html::Text(text)
    }
}

impl From<&str> for Html {
    fn from(text: &str) -> Self {
        Html::Text(text.to_string())
    }
}

/// Check if a tag is a void element (self-closing)
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag.to_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Convenient HTML builder functions
pub fn html() -> HtmlElement {
    HtmlElement::new("html")
}

pub fn head() -> HtmlElement {
    HtmlElement::new("head")
}

pub fn body() -> HtmlElement {
    HtmlElement::new("body")
}

pub fn div() -> HtmlElement {
    HtmlElement::new("div")
}

pub fn p() -> HtmlElement {
    HtmlElement::new("p")
}

pub fn h1() -> HtmlElement {
    HtmlElement::new("h1")
}

pub fn h2() -> HtmlElement {
    HtmlElement::new("h2")
}

pub fn h3() -> HtmlElement {
    HtmlElement::new("h3")
}

pub fn h4() -> HtmlElement {
    HtmlElement::new("h4")
}

pub fn h5() -> HtmlElement {
    HtmlElement::new("h5")
}

pub fn h6() -> HtmlElement {
    HtmlElement::new("h6")
}

pub fn span() -> HtmlElement {
    HtmlElement::new("span")
}

pub fn a() -> HtmlElement {
    HtmlElement::new("a")
}

pub fn img() -> HtmlElement {
    HtmlElement::self_closing("img")
}

pub fn br() -> HtmlElement {
    HtmlElement::self_closing("br")
}

pub fn hr() -> HtmlElement {
    HtmlElement::self_closing("hr")
}

pub fn input() -> HtmlElement {
    HtmlElement::self_closing("input")
}

pub fn button() -> HtmlElement {
    HtmlElement::new("button")
}

pub fn form() -> HtmlElement {
    HtmlElement::new("form")
}

pub fn ul() -> HtmlElement {
    HtmlElement::new("ul")
}

pub fn ol() -> HtmlElement {
    HtmlElement::new("ol")
}

pub fn li() -> HtmlElement {
    HtmlElement::new("li")
}

pub fn table() -> HtmlElement {
    HtmlElement::new("table")
}

pub fn tr() -> HtmlElement {
    HtmlElement::new("tr")
}

pub fn td() -> HtmlElement {
    HtmlElement::new("td")
}

pub fn th() -> HtmlElement {
    HtmlElement::new("th")
}

pub fn thead() -> HtmlElement {
    HtmlElement::new("thead")
}

pub fn tbody() -> HtmlElement {
    HtmlElement::new("tbody")
}

pub fn section() -> HtmlElement {
    HtmlElement::new("section")
}

pub fn article() -> HtmlElement {
    HtmlElement::new("article")
}

pub fn nav() -> HtmlElement {
    HtmlElement::new("nav")
}

pub fn header() -> HtmlElement {
    HtmlElement::new("header")
}

pub fn footer() -> HtmlElement {
    HtmlElement::new("footer")
}

pub fn main() -> HtmlElement {
    HtmlElement::new("main")
}

pub fn aside() -> HtmlElement {
    HtmlElement::new("aside")
}

/// Create a text node
pub fn text<S: Into<String>>(content: S) -> Html {
    Html::text(content)
}

/// Create a raw HTML node
pub fn raw<S: Into<String>>(content: S) -> Html {
    Html::raw(content)
}

/// Create an empty fragment
pub fn fragment<I>(children: I) -> Html
where
    I: IntoIterator<Item = Html>,
{
    Html::fragment(children)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_element() {
        let element = div().class("test").text("Hello, world!");
        let html = element.render();
        assert_eq!(html, r#"<div class="test">Hello, world!</div>"#);
    }

    #[test]
    fn test_self_closing_element() {
        let element = img().attr("src", "test.jpg").attr("alt", "Test");
        let html = element.render();
        assert_eq!(html, r#"<img src="test.jpg" alt="Test" />"#);
    }

    #[test]
    fn test_boolean_attribute() {
        let element = input().attr("type", "checkbox").bool_attr("checked");
        let html = element.render();
        assert_eq!(html, r#"<input type="checkbox" checked />"#);
    }

    #[test]
    fn test_nested_elements() {
        let element = div()
            .class("container")
            .child(h1().text("Title"))
            .child(p().text("Content"));

        let html = element.render();
        assert!(html.contains(r#"<div class="container">"#));
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>Content</p>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn test_text_escaping() {
        let element = div().text("<script>alert('xss')</script>");
        let html = element.render();
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn test_raw_html() {
        let element = div().raw("<em>emphasized</em>");
        let html = element.render();
        assert!(html.contains("<em>emphasized</em>"));
    }

    #[test]
    fn test_fragment() {
        let frag = fragment(vec![
            text("Hello "),
            Html::Element(span().text("world")),
            text("!"),
        ]);
        let html = frag.render();
        assert_eq!(html, "Hello <span>world</span>!");
    }

    #[test]
    fn test_multiple_classes() {
        let element = div().classes(vec!["one", "two", "three"]);
        let html = element.render();
        assert!(html.contains(r#"class="one two three""#));
    }

    #[test]
    fn test_void_elements() {
        assert!(is_void_element("br"));
        assert!(is_void_element("img"));
        assert!(is_void_element("input"));
        assert!(!is_void_element("div"));
        assert!(!is_void_element("span"));
    }

    #[test]
    fn test_text_content() {
        let element = div()
            .child(text("Hello "))
            .child(span().text("world"))
            .child(text("!"));

        let html = Html::Element(element);
        assert_eq!(html.text_content(), "Hello world!");
    }

    #[test]
    fn test_empty_html() {
        assert!(Html::empty().is_empty());
        assert!(Html::text("").is_empty());
        assert!(Html::fragment(vec![]).is_empty());
        assert!(!Html::text("content").is_empty());
    }
}
