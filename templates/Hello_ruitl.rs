// ruitl-hash: 40a04a273b2682ce507572d2800345cc
use ruitl::html::*;
use ruitl::prelude::*;
#[derive(Debug, Clone)]
pub struct HelloProps {
    pub name: String,
}
impl ComponentProps for HelloProps {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct Hello;
impl Component for Hello {
    type Props = HelloProps;
    #[allow(unused_variables)]
    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let name = &props.name;
        Ok(Html::Element(
            HtmlElement::new("div").child(Html::Element(
                HtmlElement::new("h1")
                    .child(Html::text("Hello, "))
                    .child(Html::text(&format!("{}", name)))
                    .child(Html::text("!")),
            )),
        ))
    }
}
