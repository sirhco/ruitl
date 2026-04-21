// ruitl-hash: 602148dc4f4775bdc2f44fe3d28d95e3
use ruitl::html::*;
use ruitl::prelude::*;
#[derive(Debug, Clone)]
pub struct DemoButtonProps {
    pub text: String,
    pub variant: String,
    pub href: Option<String>,
}
impl ComponentProps for DemoButtonProps {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct DemoButton;
impl Component for DemoButton {
    type Props = DemoButtonProps;
    #[allow(unused_variables)]
    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let text = &props.text;
        let variant = &props.variant;
        let href = &props.href;
        Ok(if let Some(href_val) = &href {
            Html::Element(
                HtmlElement::new("a")
                    .attr("href", &format!("{}", href_val.clone()))
                    .attr("class", &format!("{}", format!("button btn-{}", variant)))
                    .child(Html::text(&format!("{}", text.clone()))),
            )
        } else {
            Html::Element(
                HtmlElement::new("button")
                    .attr("class", &format!("{}", format!("button btn-{}", variant)))
                    .attr("type", "button")
                    .child(Html::text(&format!("{}", text.clone()))),
            )
        })
    }
}
