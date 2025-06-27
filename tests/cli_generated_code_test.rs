//! Test generated CLI code compilation and functionality
//!
//! This test verifies that the CLI-generated components compile correctly
//! and function as expected with proper variable access and advanced features.

use ruitl::component::{Component, ComponentContext};
use ruitl::html::Html;

// Include the generated components from CLI output
// Note: In a real scenario, these would be generated in the build output
#[path = "../generated-cli-v3/mod.rs"]
mod generated_components;

use generated_components::*;

#[test]
fn test_generated_hello_component() {
    let context = ComponentContext::new();
    let hello = Hello;
    let props = HelloProps {
        name: "World".to_string(),
    };

    let result = hello.render(&props, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();
    assert!(html_string.contains("World"));
    assert!(html_string.contains("<h1>"));
}

#[test]
fn test_generated_simple_if_component() {
    let context = ComponentContext::new();
    let simple_if = SimpleIf;

    // Test with show_message = true
    let props_true = SimpleIfProps { show_message: true };

    let result = simple_if.render(&props_true, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();
    assert!(html_string.contains("Hello World!"));

    // Test with show_message = false
    let props_false = SimpleIfProps {
        show_message: false,
    };

    let result = simple_if.render(&props_false, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();
    assert!(html_string.contains("No message to show"));
}

#[test]
fn test_generated_advanced_features_component() {
    let context = ComponentContext::new();
    let advanced = AdvancedFeatures;

    // Test with admin user and items
    let props = AdvancedFeaturesProps {
        title: "Admin Dashboard".to_string(),
        items: vec!["Task 1".to_string(), "Task 2".to_string()],
        show_header: true,
        user_role: "admin".to_string(),
        count: 2,
    };

    let result = advanced.render(&props, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();

    // Verify conditional rendering works
    assert!(html_string.contains("Admin Dashboard"));
    assert!(html_string.contains("Administrator"));

    // Verify loop rendering works
    assert!(html_string.contains("Task 1"));
    assert!(html_string.contains("Task 2"));

    // Verify admin controls are present
    assert!(html_string.contains("Delete"));
    assert!(html_string.contains("Add Item"));
}

#[test]
fn test_generated_advanced_features_no_header() {
    let context = ComponentContext::new();
    let advanced = AdvancedFeatures;

    // Test with header hidden
    let props = AdvancedFeaturesProps {
        title: "Hidden Header".to_string(),
        items: vec![],
        show_header: false,
        user_role: "user".to_string(),
        count: 0,
    };

    let result = advanced.render(&props, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();

    // Header should not be present
    assert!(!html_string.contains("<header"));

    // Welcome message should be shown (count = 0)
    assert!(html_string.contains("Welcome"));

    // Admin controls should not be present (user role)
    assert!(!html_string.contains("Add Item"));
}

#[test]
fn test_generated_button_component() {
    let context = ComponentContext::new();
    let button = Button;
    let props = ButtonProps {
        text: "Click Me".to_string(),
        variant: "primary".to_string(),
    };

    let result = button.render(&props, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();
    assert!(html_string.contains("Click Me"));
    assert!(html_string.contains("btn-primary"));
}

#[test]
fn test_generated_user_card_component() {
    let context = ComponentContext::new();
    let user_card = UserCard;
    let props = UserCardProps {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        role: "Developer".to_string(),
    };

    let result = user_card.render(&props, &context);
    assert!(result.is_ok());

    let html = result.unwrap();
    let html_string = html.to_string();
    assert!(html_string.contains("John Doe"));
    assert!(html_string.contains("john@example.com"));
    assert!(html_string.contains("Developer"));
}

#[test]
fn test_props_validation() {
    // Test that props structs implement required traits
    let hello_props = HelloProps {
        name: "Test".to_string(),
    };

    // Should be serializable
    let json = serde_json::to_string(&hello_props);
    assert!(json.is_ok());

    // Should be cloneable
    let cloned = hello_props.clone();
    assert_eq!(cloned.name, "Test");
}

#[test]
fn test_component_context_integration() {
    let context = ComponentContext::new();

    // All components should work with the same context
    let hello = Hello;
    let simple_if = SimpleIf;
    let advanced = AdvancedFeatures;

    let hello_props = HelloProps {
        name: "Context Test".to_string(),
    };

    let if_props = SimpleIfProps { show_message: true };

    let advanced_props = AdvancedFeaturesProps {
        title: "Context Test".to_string(),
        items: vec!["Item".to_string()],
        show_header: true,
        user_role: "user".to_string(),
        count: 1,
    };

    // All should render successfully with the same context
    assert!(hello.render(&hello_props, &context).is_ok());
    assert!(simple_if.render(&if_props, &context).is_ok());
    assert!(advanced.render(&advanced_props, &context).is_ok());
}

#[test]
fn test_variable_access_in_conditions() {
    let context = ComponentContext::new();
    let advanced = AdvancedFeatures;

    // Test various condition scenarios to ensure props access works
    let test_cases = vec![
        // Case 1: show_header = true, should show header
        (
            AdvancedFeaturesProps {
                title: "Test".to_string(),
                items: vec![],
                show_header: true,
                user_role: "user".to_string(),
                count: 0,
            },
            vec!["<header"],
        ),
        // Case 2: show_header = false, should not show header
        (
            AdvancedFeaturesProps {
                title: "Test".to_string(),
                items: vec![],
                show_header: false,
                user_role: "user".to_string(),
                count: 0,
            },
            vec!["Welcome"], // Should show welcome instead
        ),
        // Case 3: admin role, should show admin controls
        (
            AdvancedFeaturesProps {
                title: "Test".to_string(),
                items: vec!["Item".to_string()],
                show_header: true,
                user_role: "admin".to_string(),
                count: 1,
            },
            vec!["Administrator", "Delete", "Add Item"],
        ),
    ];

    for (props, expected_content) in test_cases {
        let result = advanced.render(&props, &context);
        assert!(result.is_ok());

        let html_string = result.unwrap().to_string();
        for content in expected_content {
            assert!(
                html_string.contains(content),
                "Expected '{}' in generated HTML: {}",
                content,
                html_string
            );
        }
    }
}

#[test]
fn test_loop_rendering() {
    let context = ComponentContext::new();
    let advanced = AdvancedFeatures;

    let props = AdvancedFeaturesProps {
        title: "Loop Test".to_string(),
        items: vec![
            "First Item".to_string(),
            "Second Item".to_string(),
            "Third Item".to_string(),
        ],
        show_header: true,
        user_role: "user".to_string(),
        count: 3,
    };

    let result = advanced.render(&props, &context);
    assert!(result.is_ok());

    let html_string = result.unwrap().to_string();

    // All items should be rendered
    assert!(html_string.contains("First Item"));
    assert!(html_string.contains("Second Item"));
    assert!(html_string.contains("Third Item"));

    // Should have list structure
    assert!(html_string.contains("<ul"));
    assert!(html_string.contains("<li"));

    // Count should be correct
    assert!(html_string.contains("3 items"));
}

#[test]
fn test_empty_items_handling() {
    let context = ComponentContext::new();
    let advanced = AdvancedFeatures;

    let props = AdvancedFeaturesProps {
        title: "Empty Test".to_string(),
        items: vec![], // Empty items
        show_header: true,
        user_role: "user".to_string(),
        count: 0,
    };

    let result = advanced.render(&props, &context);
    assert!(result.is_ok());

    let html_string = result.unwrap().to_string();

    // Should show welcome message instead of items
    assert!(html_string.contains("Welcome"));
    assert!(!html_string.contains("<ul")); // No list should be rendered
}
