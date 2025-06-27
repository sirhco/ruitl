//! Generated RUITL components
//! This module re-exports components from the generated directory

// Re-export all generated components from the actual generated location
#[path = "../generated/mod.rs"]
mod generated_components;

pub use generated_components::*;
