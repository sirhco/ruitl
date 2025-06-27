use ruitl::prelude::*;

#[derive(Debug, Clone)]
pub struct HelloWorldProps {
    pub message: String,
}

impl ComponentProps for HelloWorldProps {}

#[derive(Debug)]
pub struct HelloWorld;

impl Component for HelloWorld {
    type Props = HelloWorldProps;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        Ok(html! {
            <div class="component">
                <h2>{props.message}</h2>
                <p>This is a RUITL component!</p>
            </div>
        })
    }
}
