// ruitl-hash: a2ce6497990853e7004e0e4c0cd5ecad
use ruitl::html::*;
use ruitl::prelude::*;
#[derive(Debug, Clone)]
pub struct SimpleIfProps {
    pub show_message: bool,
}
impl ComponentProps for SimpleIfProps {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct SimpleIf;
impl Component for SimpleIf {
    type Props = SimpleIfProps;
    #[allow(unused_variables)]
    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let show_message = props.show_message;
        Ok(Html::Element(HtmlElement::new("div").child(
            if show_message {
                Html::Element(HtmlElement::new("p").child(Html::text("Hello World!")))
            } else {
                Html::Element(HtmlElement::new("p").child(Html::text("No message to show")))
            },
        )))
    }
}
