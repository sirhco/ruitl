// ruitl-hash: 5082fe385bf41df61895c5f2cfd37ed5
use ruitl::html::*;
use ruitl::prelude::*;
#[derive(Debug, Clone)]
pub struct DemoUserCardProps {
    pub name: String,
    pub email: String,
    pub role: String,
}
impl ComponentProps for DemoUserCardProps {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct DemoUserCard;
impl Component for DemoUserCard {
    type Props = DemoUserCardProps;
    #[allow(unused_variables)]
    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let name = &props.name;
        let email = &props.email;
        let role = &props.role;
        Ok(Html::Element(
            HtmlElement::new("div")
                .attr("class", "card")
                .child(Html::Element(
                    HtmlElement::new("h3")
                        .child(Html::text(&format!("{}", format!("User: {}", name)))),
                ))
                .child(Html::Element(HtmlElement::new("p").child(Html::text(
                    &format!("{}", format!("Email: {}", email)),
                ))))
                .child(Html::Element(
                    HtmlElement::new("p")
                        .child(Html::text(&format!("{}", format!("Role: {}", role)))),
                )),
        ))
    }
}
