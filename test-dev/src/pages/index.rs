use ruitl::prelude::*;
use crate::components::HelloWorld;

#[derive(Debug, Clone)]
pub struct IndexProps {}

impl ComponentProps for IndexProps {}

#[derive(Debug)]
pub struct Index;

impl Component for Index {
    type Props = IndexProps;

    fn render(&self, _props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        Ok(html! {
            <div>
                <h1>Welcome to RUITL!</h1>
                <HelloWorld message="Hello from RUITL!" />
            </div>
        })
    }
}
