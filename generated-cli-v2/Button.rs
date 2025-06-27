use ruitl::html::*;
use ruitl::prelude::*;
use std::collections::HashMap;
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct ButtonProps {
    pub text: String,
    pub variant: Option<String>,
}
impl ruitl::component::ComponentProps for ButtonProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct Button;
impl ruitl::component::Component for Button {
    type Props = ButtonProps;
    fn render(
        &self,
        props: &Self::Props,
        context: &ruitl::component::ComponentContext,
    ) -> ruitl::error::Result<ruitl::html::Html> {
        Ok(ruitl::html::Html::Element(
            ruitl::html::HtmlElement::new("button")
                .attr("class", &format!("{}", format!("btn btn-{}", variant)))
                .attr("type", "button")
                .child(ruitl::html::Html::text(&format!("{}", props.text))),
        ))
    }
}
