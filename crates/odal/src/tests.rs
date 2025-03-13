use std::fs;
use std::io::Write;
use tempfile::tempdir;
use toml::Table;

use crate::{Config, extract_diff};

use serde::{Deserialize, Serialize};

// Test configurations for our unit tests
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
struct TestConfig {
    string_value: String,
    int_value: i32,
    nested: NestedConfig,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
struct NestedConfig {
    setting_a: String,
    setting_b: i32,
    deeper: DeeperConfig,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
struct DeeperConfig {
    flag: bool,
    value: f64,
}

impl Config for TestConfig {
    const PATH: &'static str = "test_config.toml";
}

// Test the extract_diff function
#[test]
fn test_extract_diff() {
    // Create two TOML configurations
    let main_toml = r#"
        string_value = "original"
        int_value = 42
        
        [nested]
        setting_a = "hello"
        setting_b = 100
        
        [nested.deeper]
        flag = true
        value = 3.14
    "#;
    
    let changed_toml = r#"
        string_value = "changed"
        int_value = 42
        
        [nested]
        setting_a = "hello"
        setting_b = 200
        
        [nested.deeper]
        flag = false
        value = 3.14
    "#;
    
    // Parse into TOML Tables
    let main_table: Table = main_toml.parse().unwrap();
    let changed_table: Table = changed_toml.parse().unwrap();
    
    // Extract differences
    let diff_table = extract_diff(&main_table, &changed_table);
    
    // Verify the diff contains only what changed
    assert!(diff_table.contains_key("string_value"));
    assert!(!diff_table.contains_key("int_value")); // unchanged value
    
    assert!(diff_table.contains_key("nested"));
    let nested = diff_table.get("nested").unwrap().as_table().unwrap();
    assert!(!nested.contains_key("setting_a")); // unchanged value
    assert!(nested.contains_key("setting_b")); // changed value
    
    assert!(nested.contains_key("deeper"));
    let deeper = nested.get("deeper").unwrap().as_table().unwrap();
    assert!(deeper.contains_key("flag")); // changed value
    assert!(!deeper.contains_key("value")); // unchanged value
}

// Test the full overlay saving functionality
#[test]
fn test_save_as_overlay() {
    // Create a temporary directory for our test
    let temp_dir = tempdir().unwrap();
    let config_dir = temp_dir.path().join("config");
    let overlay_dir = temp_dir.path().join("config/overlay/test_robot");
    
    // Create directories
    fs::create_dir_all(&config_dir).unwrap();
    fs::create_dir_all(&overlay_dir).unwrap();
    
    // Create a main config file
    let main_config_path = config_dir.join(TestConfig::PATH);
    let main_config_content = r#"
string_value = "original"
int_value = 42

[nested]
setting_a = "hello"
setting_b = 100

[nested.deeper]
flag = true
value = 3.14
"#;
    
    let mut file = fs::File::create(&main_config_path).unwrap();
    file.write_all(main_config_content.as_bytes()).unwrap();
    
    // Load the main config
    let main_config = TestConfig::load(&config_dir).unwrap();
    
    // Create a modified config
    let mut modified_config = main_config.clone();
    modified_config.string_value = "changed".to_string();
    modified_config.nested.setting_b = 200;
    modified_config.nested.deeper.flag = false;
    
    // Save the overlay
    modified_config.save_as_overlay(&main_config, &overlay_dir).unwrap();
    
    // Verify the overlay file exists
    let overlay_config_path = overlay_dir.join(TestConfig::PATH);
    assert!(overlay_config_path.exists());
    
    // Load the overlay file and verify it contains only the changes
    let overlay_content = fs::read_to_string(&overlay_config_path).unwrap();
    let overlay_table: Table = overlay_content.parse().unwrap();
    
    assert_eq!(overlay_table.get("string_value").unwrap().as_str().unwrap(), "changed");
    assert!(!overlay_table.contains_key("int_value")); // unchanged value
    
    let nested = overlay_table.get("nested").unwrap().as_table().unwrap();
    assert!(!nested.contains_key("setting_a")); // unchanged value
    assert_eq!(nested.get("setting_b").unwrap().as_integer().unwrap(), 200);
    
    let deeper = nested.get("deeper").unwrap().as_table().unwrap();
    assert_eq!(deeper.get("flag").unwrap().as_bool().unwrap(), false);
    assert!(!deeper.contains_key("value")); // unchanged value
    
    // Test loading with overlay
    let config_with_overlay = TestConfig::load_with_overlay(&config_dir, &overlay_dir).unwrap();
    
    // Verify the overlay was applied correctly
    assert_eq!(config_with_overlay.string_value, "changed");
    assert_eq!(config_with_overlay.int_value, 42);
    assert_eq!(config_with_overlay.nested.setting_a, "hello");
    assert_eq!(config_with_overlay.nested.setting_b, 200);
    assert_eq!(config_with_overlay.nested.deeper.flag, false);
    assert_eq!(config_with_overlay.nested.deeper.value, 3.14);
}