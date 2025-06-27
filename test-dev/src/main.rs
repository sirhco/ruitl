use ruitl::prelude::*;
use std::collections::HashMap;

mod components;
mod pages;

use components::HelloWorld;
use pages::Index;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize RUITL
    ruitl::init()?;

    // Create renderer
    let renderer_config = RendererConfig::default();
    let renderer = UniversalRenderer::new(renderer_config);

    // Register components
    renderer.register_component("HelloWorld", HelloWorld).await;
    renderer.register_component("Index", Index).await;

    // Create context
    let context = RenderContext::new()
        .with_path("/")
        .with_target(RenderTarget::Development);

    // Render index page
    let options = RenderOptions::new();
    let html = renderer.render(&context, &options).await?;

    println!("{}", html);

    Ok(())
}
