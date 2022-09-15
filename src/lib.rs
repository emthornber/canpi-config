//! # canpi-config
//!
//! This crate provides functionality to read and write the canpi server configuration
//! and to define which configuration items can be changed or viewed by the user and which are hidden.
//!
//! There is a JSON file that defines the configuration item format and default values
//! along with a matching schema file.  This file is loaded to the ConfigHash.  The canpi INI file,
//! if it exists, is read to update current value of the configuration items so the ConfigHash
//! becomes the single source of truth.
//!
//! There is a function to write the ConfigHash as an INI file.
//
//  30 November, 2021 - E M Thornber
//

use ini::Ini;

use jsonschema::{Draft, JSONSchema};
use serde::Deserialize;

use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::string::String;

use thiserror::Error;

/// Type alias for a HashMap
pub type ConfigHash = HashMap<String, Attribute>;

#[derive(Clone, Deserialize, Debug, PartialEq)]
/// Defines the possible behaviours of an attribute
pub enum AttributeAction {
    /// User can update the attribute value
    Edit,
    /// User can see the current value of the attribute but cannot change it
    Display,
    /// Attribute is for internal use only
    Hide,
}

#[derive(Clone, Deserialize, Debug)]
/// Definition of an attribute
pub struct Attribute {
    // Text used to label edit box on form
    prompt: String,
    // Text displayed when cursor hovers over edit box
    tooltip: String,
    // Current value of attribute.  Used to populate .cfg file
    current: String,
    // Default value of attribute
    default: String,
    // Regular expression to validate user input
    format: String,
    // How attribute behaves
    action: AttributeAction,
}

#[derive(Error, Debug)]
/// Categorizes the cause of errors when processing the configuration files
pub enum CanPiCfgError {
    /// The error was caused by a failure to read the configuration file
    #[error("cannot open configuration file")]
    Io(#[from] std::io::Error),
    /// The error was caused by failure to validate JSON input
    #[error("JSON input '{0}' failed to validate against schema")]
    JsonSchema(String),
    /// The error was caused by a failure to deserialize the JSON
    #[error("cannot deserialize configuration file")]
    Json(#[from] serde_json::Error),
    /// The error was caused when reading or writing the .cfg file
    #[error("cannot read cfg file")]
    Ini(#[from] ini::Error),
}

/// Read JSON file and return Serde Value
fn read_json_file<P: AsRef<Path>>(json_path: P) -> Result<Value, CanPiCfgError> {
    let mut json_string = String::new();
    File::open(json_path)
        .unwrap()
        .read_to_string(&mut json_string)
        .unwrap();
    let json_value: Value = serde_json::from_str(json_string.as_str())?;
    Ok(json_value)
}

/// Validate configuration definition JSON against the JSON schema
pub fn validate_defn_file(json_schema: Value, json_defn: Value) -> bool {
    let compiled_schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&json_schema)
        .expect("A valid schema");
    compiled_schema.is_valid(&json_defn)
}
/// Read the contents of a file as JSON and load into an instance of 'ConfigHash'
pub fn read_defn_file<P: AsRef<Path>>(path: P) -> Result<ConfigHash, CanPiCfgError> {
    // Open the file in read-only mode with buffer
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of 'ConfigHash'.
    let config = serde_json::from_reader(reader)?;

    // Return the 'ConfigHash'.
    Ok(config)
}

/// Read the contents of a string as JSON and load into an instance of 'ConfigHash'
pub fn read_defn_str(data: &str) -> Result<ConfigHash, CanPiCfgError> {
    let config = serde_json::from_str(data)?;

    // Return the 'ConfigHash'.
    Ok(config)
}

/// Filters the attributes by action
pub fn attributes_with_action(attrs: &ConfigHash, action: AttributeAction) -> ConfigHash {
    let mut attr2 = ConfigHash::new();
    attr2.extend(
        attrs
            .iter()
            .filter(|(_k, v)| v.action == action)
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    attr2
}

/// Read the current canpi cfg values from file defined by 'path'
pub fn read_cfg_file<P: AsRef<Path>>(path: P) -> Result<Ini, CanPiCfgError> {
    let cfg = Ini::load_from_file(path)?;
    Ok(cfg)
}
/// Output the keys and current values of items to the file defined by 'path'
pub fn write_cfg_file<P: AsRef<Path>>(path: P, config: ConfigHash) -> Result<(), CanPiCfgError> {
    let mut cfg = Ini::new();
    for (k, v) in config {
        cfg.set_to(None::<String>, k.clone(), v.current.clone());
    }
    cfg.write_to_file(path)?;
    Ok(())
}

fn update_current_value(mut a: Attribute, v: String) -> Attribute {
    a.current = v;
    a
}

/// Read the INI format file 'path' and load the values into the 'current' field of the matching
/// ConfigHash entry.
pub fn update_defn_from_cfg<P: AsRef<Path>>(
    path: P,
    config: ConfigHash,
) -> Result<ConfigHash, CanPiCfgError> {
    let cfg = Ini::load_from_file(path)?;
    let mut c = config.clone();
    let properties = cfg.section(None::<String>);
    if let Some(p) = properties {
        for (k, v) in p.iter() {
            let attr = config.get(k);
            if let Some(a) = attr {
                c.insert(
                    k.to_string(),
                    update_current_value(a.clone(), v.to_string()),
                );
            } else {
                println!("Key '{}' not defined in configuration", k);
            }
        }
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use serde_json::Result;
    use std::env;

    #[test]
    fn single_attribute() -> Result<()> {
        // Some JSON input data as a &str.  Maybe this comes from a file.
        let data = r#"
        {
            "prompt": "CAN Id",
            "tooltip": "The CAN Id used by the CANPi CAP/Zero on the CBUS",
            "current": "100",
            "default": "100",
            "format": "[0-9]{1,4}",
            "action": "Display"
        }"#;

        // Parse the string of data into an Attribute object.
        let a: Attribute = serde_json::from_str(data)?;

        // println!("Attribute is {} ({})", a.attribute, a.tooltip);
        assert_eq!(a.action, AttributeAction::Display);
        Ok(())
    }

    #[test]
    fn single_vector() {
        let data = r#"
        {
                  "canid" : {
                      "prompt": "CAN Id",
                      "tooltip": "The CAN Id used by the CAN Pi CAP/Zero on the CBUS",
                      "current": "100",
                      "default": "100",
                      "format": "[0-9]{1,4}",
                      "action": "Display"
                  },
                  "node_number" : {
                      "prompt": "Node Number",
                      "tooltip": "Module Node Number - change your peril",
                      "current": "4321",
                      "default": "4321",
                      "format": "[0-9]{1,4}",
                      "action": "Display"
                  },
                  "start_event_id" : {
                      "prompt": "Start Event Id",
                      "tooltip": "The event that will be generated when the ED and GridConnect services start (ON) and stop (OFF)",
                      "current": "1",
                      "default": "1",
                      "format": "[0-9]{1,2}",
                      "action": "Edit"
                  },
                  "node_mode" : {
                      "prompt": "",
                      "tooltip": "",
                      "current": "0",
                      "default": "0",
                      "format": "[0-9]{1,2}",
                      "action": "Hide"
                  }
        }"#;
        let config: ConfigHash = read_defn_str(data).expect("Deserialize failed");
        assert_eq!(config.len(), 4);
        let displayable: ConfigHash = attributes_with_action(&config, AttributeAction::Display);
        assert_eq!(displayable.len(), 2);
        assert!(displayable.contains_key("canid"));
        assert!(displayable.contains_key("node_number"));
        let editable: ConfigHash = attributes_with_action(&config, AttributeAction::Edit);
        assert_eq!(editable.len(), 1);
        assert!(editable.contains_key("start_event_id"));
        let hidden: ConfigHash = attributes_with_action(&config, AttributeAction::Hide);
        assert_eq!(hidden.len(), 1);
        assert!(hidden.contains_key("node_mode"));
    }

    #[test]
    #[should_panic]
    fn single_mal_formed_vector() {
        let data = r#"
        {
                  "canid" : {
                      "prompt": "CAN Id",
                      "tooltip": "The CAN Id used by the CAN Pi CAP/Zero on the CBUS",
                      "current": "100",
                      "default": "100",
                      "format": "[0-9]{1,4}",
                      "action": "Display"
                  },
                  "node_number" : {
                      "prompt": "Node Number",
                      "tooltip": "Module Node Number - change your peril",
                      "current": "4321",
                      "default": "4321",
                      "format": "[0-9]{1,4}",
                      "action": "Display"
                  },
                  "start_event_id" : {
                      "prompt": "Start Event Id",
                      "tooltip": "The event that will be generated when the ED and GridConnect services start (ON) and stop (OFF)",
                      "current": "1",
                      "default": "1",
                      "format": "[0-9]{1,2}",
                      "action": "Edit"
                  },
                  "node_mode" : {
                      "tooltip": "",
                      "current": "0",
                      "default": "0",
                      "format": "[0-9]{1,2}",
                      "action": "Hide"
                  }
        }"#;
        // Should fail as key 'node_mode' is missing a 'prompt' value
        let _config: ConfigHash = read_defn_str(data).expect("Deserialize failed");
    }

    #[test]
    fn read_json_file_good() -> std::result::Result<(), String> {
        dotenv().ok();
        let config_file = env::var("CONFIG_FILE").expect("CONFIG_FILE is not set in .env file");
        let _config = read_json_file(config_file).expect("Reading JSON failed");
        Ok(())

    }
    #[test]
    fn read_defn_file_good() {
        dotenv().ok();
        let config_file = env::var("CONFIG_FILE").expect("CONFIG_FILE is not set in .env file");
        let config = read_defn_file(config_file).expect("Deserialize failed");
        assert_eq!(config.len(), 29)
    }

    #[test]
    #[should_panic]
    fn write_init_file() {
        dotenv().ok();
        let config_file = env::var("CONFIG_FILE").expect("CONFIG_FILE is not set in .env file");
        let config = read_defn_file(config_file).expect("Deserialize failed");
        let cfg_file = "/bert/fred/joe.ini".to_string();
        write_cfg_file(cfg_file, config).expect("Failed to write cfg file");
    }
}
