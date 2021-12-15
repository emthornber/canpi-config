//
// Configuration data manipulation
//
// There is a JSON file that defines the configuration file format and default values along with a
// matching schema file.
//
// There are functions to read and write the config as an INI file
//
//  30 November, 2021 - E M Thornber
//

//use serde::de::value::MapAccessDeserializer;
use serde::Deserialize;
use serde_json::Result;
use serde_json::Value;

use std::collections::HashMap;

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

pub fn read_defn_file(file_path: )
pub fn read_defn_str(data: &str) -> Result<ConfigHash> {
    let c = serde_json::from_str(data);
    let c = match c {
        Ok(hash) => hash,
        Err(e) => panic!("Problem deserializing config definition file: {:?}", e),
    };
    Ok(c)
}

pub fn attributes_with_action(
    attrs: ConfigHash,
    action: AttributeAction
) -> ConfigHash {
    let mut attr2 = ConfigHash::new();
    attr2.extend(attrs
        .iter()
        .filter(|(_k, v)| v.action == action)
        .map(|(k, v)| (k.clone(), v.clone()))
    );
    attr2
}

#[cfg(test)]
mod tests {
    use crate::{Attribute, ConfigHash, AttributeAction, read_defn_str, attributes_with_action};
    use serde_json::Result;

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
}
