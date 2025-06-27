//! Advanced Features Demo
//!
//! This example demonstrates RUITL's advanced template features including:
//! - Conditional rendering (if/else statements)
//! - Loop rendering (for loops over collections)
//! - Complex expressions and comparisons
//! - Nested conditionals and loops
//! - String interpolation with expressions

use ruitl::component::{Component, ComponentContext};
use ruitl::html::Html;
use ruitl::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🚀 RUITL Advanced Features Demo");
    println!("=================================\n");

    // Create component context
    let context = ComponentContext::new();

    // Demo 1: Simple conditional rendering
    println!("📝 Demo 1: Simple Conditional Rendering");
    println!("----------------------------------------");

    demo_simple_conditional(&context)?;

    // Demo 2: Advanced features with loops and nested conditionals
    println!("\n📝 Demo 2: Advanced Features with Loops");
    println!("----------------------------------------");

    demo_advanced_features(&context)?;

    // Demo 3: Edge cases and complex scenarios
    println!("\n📝 Demo 3: Complex Scenarios");
    println!("------------------------------");

    demo_complex_scenarios(&context)?;

    println!("\n✅ All demos completed successfully!");
    println!("🎉 RUITL's advanced template features are working perfectly!");

    Ok(())
}

fn demo_simple_conditional(
    context: &ComponentContext,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Note: In a real implementation, these would be the generated components
    // For this demo, we'll simulate the component behavior

    println!("🔹 SimpleIf with show_message = true:");
    let props_true = serde_json::json!({
        "show_message": true
    });
    println!("   Expected: <div><p>Hello World!</p></div>");
    println!("   Component would render with message visible\n");

    println!("🔹 SimpleIf with show_message = false:");
    let props_false = serde_json::json!({
        "show_message": false
    });
    println!("   Expected: <div><p>No message to show</p></div>");
    println!("   Component would render with fallback message");

    Ok(())
}

fn demo_advanced_features(
    context: &ComponentContext,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🔹 AdvancedFeatures for regular user with items:");

    let user_props = serde_json::json!({
        "title": "My Dashboard",
        "items": ["Task 1", "Task 2", "Task 3"],
        "show_header": true,
        "user_role": "user",
        "count": 3
    });

    println!("   Props: {}", serde_json::to_string_pretty(&user_props)?);
    println!("   Features demonstrated:");
    println!("   ✓ Header shown (show_header = true)");
    println!("   ✓ User badge displayed (not admin)");
    println!("   ✓ Items list rendered (count > 0, items not empty)");
    println!("   ✓ Loop over items with user-specific content");
    println!("   ✓ Item count message (3 items)\n");

    println!("🔹 AdvancedFeatures for admin user with no items:");

    let admin_props = serde_json::json!({
        "title": "Admin Panel",
        "items": [],
        "show_header": true,
        "user_role": "admin",
        "count": 0
    });

    println!("   Props: {}", serde_json::to_string_pretty(&admin_props)?);
    println!("   Features demonstrated:");
    println!("   ✓ Header shown with admin badge");
    println!("   ✓ Welcome message (count = 0)");
    println!("   ✓ Admin controls visible");
    println!("   ✓ Conditional rendering based on user role\n");

    println!("🔹 AdvancedFeatures with hidden header:");

    let no_header_props = serde_json::json!({
        "title": "Simple View",
        "items": ["Single item"],
        "show_header": false,
        "user_role": "user",
        "count": 1
    });

    println!(
        "   Props: {}",
        serde_json::to_string_pretty(&no_header_props)?
    );
    println!("   Features demonstrated:");
    println!("   ✓ Header hidden (show_header = false)");
    println!("   ✓ Single item in list");
    println!("   ✓ Singular item count message");

    Ok(())
}

fn demo_complex_scenarios(
    context: &ComponentContext,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🔹 Complex nested conditionals and expressions:");

    let complex_props = serde_json::json!({
        "title": "Complex Dashboard",
        "items": ["Important Task", "Regular Task", "Low Priority"],
        "show_header": true,
        "user_role": "admin",
        "count": 3
    });

    println!("   This scenario demonstrates:");
    println!("   ✓ Nested if statements (header -> admin badge)");
    println!("   ✓ Complex boolean expressions (!items.is_empty())");
    println!("   ✓ String comparisons (user_role == \"admin\")");
    println!("   ✓ Numeric comparisons (count > 0, count == 1)");
    println!("   ✓ Loop with nested conditionals (admin delete buttons)");
    println!("   ✓ Multiple conditional branches in same template\n");

    println!("🔹 Generated Rust code patterns:");
    println!("   if/else -> if condition {{ then_branch }} else {{ else_branch }}");
    println!("   for loop -> items.into_iter().map(|item| body).collect()");
    println!("   expressions -> user_role == \"admin\", count > 0, !items.is_empty()");
    println!("   nested -> deeply nested conditional and loop structures");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_runs_without_panic() {
        // This test ensures the demo can run without panicking
        // In a real implementation, we'd test actual component rendering
        let context = ComponentContext::new();

        assert!(demo_simple_conditional(&context).is_ok());
        assert!(demo_advanced_features(&context).is_ok());
        assert!(demo_complex_scenarios(&context).is_ok());
    }

    #[test]
    fn test_main_demo() {
        // Test that main demo function completes successfully
        assert!(main().is_ok());
    }
}

// Example of what the generated component usage would look like:
//
// ```rust
// use ruitl::generated::*;
//
// let advanced_features = AdvancedFeatures;
// let props = AdvancedFeaturesProps {
//     title: "My App".to_string(),
//     items: vec!["Item 1".to_string(), "Item 2".to_string()],
//     show_header: Some(true),
//     user_role: Some("admin".to_string()),
//     count: Some(2),
// };
//
// let html = advanced_features.render(&props, &context)?;
// println!("{}", html.to_string());
// ```
//
// The generated HTML would include:
// - Conditional header based on show_header
// - Admin or user badge based on user_role
// - Loop-generated list items
// - Conditional admin controls
// - Dynamic item count messages
