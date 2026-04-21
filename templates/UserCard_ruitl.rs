// ruitl-hash: 86ac47347c69d21b6fb17aa310d84f0f
use ruitl::html::*;
use ruitl::prelude::*;
#[derive(Debug, Clone)]
pub struct UserCardProps {
    pub name: String,
    pub email: String,
    pub role: String,
}
impl ComponentProps for UserCardProps {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct UserCard;
impl Component for UserCard {
    type Props = UserCardProps;
    #[allow(unused_variables)]
    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        let name = &props.name;
        let email = &props.email;
        let role = &props.role;
        Ok(Html::Element(
            HtmlElement::new("div")
                .attr("class", "user-card")
                .child(Html::Element(
                    HtmlElement::new("div")
                        .attr("class", "user-header")
                        .child(Html::Element(
                            HtmlElement::new("h3")
                                .attr("class", "user-name")
                                .child(Html::text(&format!("{}", name))),
                        ))
                        .child(Html::Element(
                            HtmlElement::new("span")
                                .attr("class", "user-role")
                                .child(Html::text(&format!("{}", role))),
                        )),
                ))
                .child(Html::Element(
                    HtmlElement::new("div")
                        .attr("class", "user-contact")
                        .child(Html::Element(
                            HtmlElement::new("p")
                                .attr("class", "user-email")
                                .child(Html::text(&format!("{}", email))),
                        )),
                )),
        ))
    }
}
