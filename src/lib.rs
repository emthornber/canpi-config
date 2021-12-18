//
// Configuration data manipulation
//
// There is a JSON file that defines the configuration file format and values (default & current)
// along with a matching schema file.
//
// There are functions to read and write the config as an INI file
//
//  30 November, 2021 - E M Thornber
//

use ini::Ini;

use serde::Deserialize;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use thiserror::Error;

type ConfigHash = HashMap<String, Attribute>;

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub enum AttributeAction {
    Edit,
    Display,
    Hide,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Attribute {
    prompt: String,
    tooltip: String,
    current: String,
    default: String,
    format: String,
    action: AttributeAction,
}

#[derive(Error, Debug)]
pub enum CanPiCfgError {
    #[error("cannot open configuration file")]
    IoError(#[from] std::io::Error),
    #[error("cannot deserialize configuration file")]
    JsonError(#[from] serde_json::Error),
    #[error("cannot read cfg file")]
    IniError(#[from] ini::Error),
}

pub fn read_defn_file<P: AsRef<Path>>(path: P) -> Result<ConfigHash, CanPiCfgError> {
    // Open the file in read-only mode with buffer
    let file = File::open(path)?;
    /*    let file = match file {
            Ok(f) => f,
            Err(e) => panic!("Problem opening config definition file: {:?}", e),
        };*/
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of 'ConfigHash'.
    let config = serde_json::from_reader(reader)?;
    /*    let c = match c {
            Ok(hash) => hash,
            Err(e) => panic!("Problem deserializing config definition file: {:?}", e),
        };*/

    // Return the 'ConfigHash'.
    Ok(config)
}

pub fn read_defn_str(data: &str) -> Result<ConfigHash, CanPiCfgError> {
    let config = serde_json::from_str(data)?;
    /*    let c = match c {
            Ok(hash) => hash,
            Err(e) => panic!("Problem deserializing config definition str: {:?}", e),
        };*/

    // Return the 'ConfigHash'.
    Ok(config)
}

pub fn attributes_with_action(
    attrs: ConfigHash,
    action: AttributeAction,
) -> ConfigHash {
    let mut attr2 = ConfigHash::new();
    attr2.extend(attrs
        .iter()
        .filter(|(_k, v)| v.action == action)
        .map(|(k, v)| (k.clone(), v.clone()))
    );
    attr2
}

pub fn write_cfg_file<P: AsRef<Path>>(path: P, config: ConfigHash) -> Result<(), CanPiCfgError> {
    let mut cfg = Ini::new();
    for (k, v) in config {
        cfg.set_to(None::<String>, k.clone(), v.current.clone());
    }
    cfg.write_to_file(path)?;
    /*   let c = match c {
           Ok(ini) => ini,
           Err(e) => panic!("Problem writing cfg file: {:?}", e),
       };*/
    Ok(())
}

pub fn update_current(mut a: Attribute, v: String) -> Attribute {
    a.current = v;
    a
}

pub fn update_config_from_cfg<P: AsRef<Path>>(path: P, config: ConfigHash) -> Result<ConfigHash, CanPiCfgError> {
    let cfg = Ini::load_from_file(path)?;
    let mut c = config.clone();
    for (k, v) in cfg.general_section().iter() {
        let attr = config.get(k);
        if let Some(a) = attr {
            c.insert(k.to_string(), update_current(a.clone(), v.to_string()));
        } else {
            println!("Key '{}' not defined in configuration", k);
        }
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use crate::{Attribute, AttributeAction, ConfigHash, attributes_with_action, read_defn_str, read_defn_file, write_cfg_file};
    use dotenv::dotenv;
    use serde_json::Result;
    use std::env;

    #[test]
    fn single_attribute() -> Result<()> {
        // Some JSON input data as a &str.  Maybe this comes from a file.
        let data = r#"
        {
            "prompt": "CAN Id",
            "tooltip": "The CAN Id used by the CAN Pi CAP/Zero on the CBUS",
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
        let displayable: ConfigHash = attributes_with_action(config.clone(), AttributeAction::Display);
        assert_eq!(displayable.len(), 2);
        let editable: ConfigHash = attributes_with_action(config.clone(), AttributeAction::Edit);
        assert_eq!(editable.len(), 1);
        let hidden: ConfigHash = attributes_with_action(config.clone(), AttributeAction::Hide);
        assert_eq!(hidden.len(), 1);
    }

    #[test]
    fn read_json_file() {
        dotenv().ok();
        let config_file = env::var("CONFIG_FILE").expect("CONFIG_FILE is not set in .env file");
        let config = read_defn_file(config_file).expect("Deserialize failed");
        assert_eq!(config.len(), 29);
    }

    #[test]
    #[should_panic]
    fn read_json_file_2() {
        dotenv().ok();
        let _config_file = env::var("CONFOG_FILE").expect("CONFIG_FILE is not set in .env file");
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
