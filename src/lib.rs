//
// Configuration data manipulation
//
// There is a JSON file that defines the configuration file format and default values along with a
// matching schema file.
//
// There are functions to read and write the config as an INI file with sections
//
//  30 November, 2021 - E M Thornber
//

use serde::Deserialize;
use serde_json::Result;
use serde_json::Value;

use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(flatten)]
    map: HashMap<String, Attribute>,
}

#[derive(Deserialize, Debug)]
struct Attribute {
    prompt: String,
    tooltip: String,
    current: String,
    default: String,
    format: String,
    action: String,
}

impl Config {
    pub fn new(data: &str) -> Result<Config> {
        let c = serde_json::from_str(data);
        let c = match c {
            Ok(hash) => hash,
            Err(e) => panic!("Problem deserializing config file: {:?}", e),
        };
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Attribute, Config};
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
        assert_eq!(a.action, "Display");
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
                      "action": "Hidden"
                  }
        }"#;
        let mut config: Config = Config::new(&data).expect("Deserialize failed");
        assert_eq!(config.map.len(), 4);
    }
}
