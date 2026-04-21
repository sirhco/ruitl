// ruitl-hash: 37803d88c19264fa99b0ea8126bb6cc5
use ruitl::html::*;
use ruitl::prelude::*;
#[derive(Debug, Clone)]
pub struct ButtonProps {
    pub text: String,
    pub variant: String,
}
impl ComponentProps for ButtonProps {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct Button;
impl Component for Button {
    type Props = ButtonProps;
    #[allow(unused_variables)]
    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let text = &props.text;
        let variant = &props.variant;
        Ok(Html::Element(
            HtmlElement::new("button")
                .attr("class", &format!("{}", format!("btn btn-{}", variant)))
                .attr("type", "button")
                .child(Html::text(&format!("{}", text))),
        ))
    }
}
