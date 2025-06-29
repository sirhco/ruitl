use ruitl::html::*;
use ruitl::prelude::*;
use std::collections::HashMap;
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct UserCardProps {
    pub name: String,
    pub email: String,
    pub role: String,
}
impl ruitl::component::ComponentProps for UserCardProps {
    fn validate(&self) -> ruitl::error::Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct UserCard;
impl ruitl::component::Component for UserCard {
    type Props = UserCardProps;
    fn render(
        &self,
        props: &Self::Props,
        context: &ruitl::component::ComponentContext,
    ) -> ruitl::error::Result<ruitl::html::Html> {
        let name = &props.name;
        let email = &props.email;
        let role = &props.role;
        Ok(ruitl::html::Html::Element(
            ruitl::html::HtmlElement::new("div")
                .attr("class", "user-card")
                .child(ruitl::html::Html::Element(
                    ruitl::html::HtmlElement::new("div")
                        .attr("class", "user-header")
                        .child(ruitl::html::Html::Element(
                            ruitl::html::HtmlElement::new("h3")
                                .attr("class", "user-name")
                                .child(ruitl::html::Html::text(&format!("{}", name))),
                        ))
                        .child(ruitl::html::Html::Element(
                            ruitl::html::HtmlElement::new("span")
                                .attr("class", "user-role")
                                .child(ruitl::html::Html::text(&format!("{}", role))),
                        )),
                ))
                .child(ruitl::html::Html::Element(
                    ruitl::html::HtmlElement::new("div")
                        .attr("class", "user-contact")
                        .child(ruitl::html::Html::Element(
                            ruitl::html::HtmlElement::new("p")
                                .attr("class", "user-email")
                                .child(ruitl::html::Html::text(&format!("{}", email))),
                        )),
                )),
        ))
    }
}
