use ruitl::html::*;
use ruitl::prelude::*;
use std::collections::HashMap;
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct HelloProps {
    pub name: String,
}
impl ruitl::component::ComponentProps for HelloProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct Hello;
impl ruitl::component::Component for Hello {
    type Props = HelloProps;
    fn render(
        &self,
        props: &Self::Props,
        context: &ruitl::component::ComponentContext,
    ) -> ruitl::error::Result<ruitl::html::Html> {
        let name = &props.name;
        Ok(ruitl::html::Html::Element(
            ruitl::html::HtmlElement::new("div").child(ruitl::html::Html::Element(
                ruitl::html::HtmlElement::new("h1")
                    .child(ruitl::html::Html::text("Hello, "))
                    .child(ruitl::html::Html::text(&format!("{}", name)))
                    .child(ruitl::html::Html::text("!")),
            )),
        ))
    }
}
