use ruitl::html::*;
use ruitl::prelude::*;
use std::collections::HashMap;
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct SimpleIfProps {
    pub show_message: bool,
}
impl ruitl::component::ComponentProps for SimpleIfProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct SimpleIf;
impl ruitl::component::Component for SimpleIf {
    type Props = SimpleIfProps;
    fn render(
        &self,
        props: &Self::Props,
        context: &ruitl::component::ComponentContext,
    ) -> ruitl::error::Result<ruitl::html::Html> {
        let show_message = props.show_message;
        Ok(ruitl::html::Html::Element(
            ruitl::html::HtmlElement::new("div").child(if show_message {
                ruitl::html::Html::Element(
                    ruitl::html::HtmlElement::new("p")
                        .child(ruitl::html::Html::text("Hello World!")),
                )
            } else {
                ruitl::html::Html::Element(
                    ruitl::html::HtmlElement::new("p")
                        .child(ruitl::html::Html::text("No message to show")),
                )
            }),
        ))
    }
}
