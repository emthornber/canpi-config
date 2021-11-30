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

use serde::{Deserialize};
use serde_json::{Error, Result, Value, from_str};

#[derive(Deserialize)]
struct Config {
    cbus: Vec<Attribute>,
    general: Vec<Attribute>,
    wifi: Vec<Attribute>,
}

#[derive(Deserialize)]
struct Attribute {
    attribute: String,
    prompt: String,
    tooltip: String,
    default: String,
    format: String,
    action: String,
}

impl Config {
    pub fn new(data: &str) -> Result<Config> {
        let c = serde_json::from_str( data);
        let mut c = match c {
            Ok(config) => return config,
            Err(e) => return Err(e),
        };
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
            "attribute": "canid",
            "prompt": "CAN Id",
            "tooltip": "The CAN Id used by the CAN Pi CAP/Zero on the CBUS",
            "default": "100",
            "format": "[0-9]{1,4}",
            "action": "Display"
        }"#;

        // Parse the string of data into an Attribute object.
        let a: Attribute = serde_json::from_str(data)?;

        // println!("Attribute is {} ({})", a.attribute, a.tooltip);
        assert_eq!(a.attribute, "canid");
        Ok(())
    }

    #[test]
    fn single_vector() {
        let data = r#"
        {
            "cbus": [
                {
                  "attribute": "canid",
                  "prompt": "CAN Id",
                  "tooltip": "The CAN Id used by the CAN Pi CAP/Zero on the CBUS",
                  "default": "100",
                  "format": "[0-9]{1,4}",
                  "action": "Display"
                },
                {
                  "attribute": "node_number",
                  "prompt": "Node Number",
                  "tooltip": "Module Node Number - change your peril",
                  "default": "4321",
                  "format": "[0-9]{1,4}",
                  "action": "Display"
                },
                {
                  "attribute": "start_event_id",
                  "prompt": "Start Event Id",
                  "tooltip": "The event that will be generated when the ED and GridConnect services start (ON) and stop (OFF)",
                  "default": "1",
                  "format": "[0-9]{1,2}",
                  "action": "Edit"
                },
                {
                  "attribute":"node_mode",
                  "prompt": "",
                  "tooltip": "",
                  "default": "0",
                  "format": "[0-9]{1,2}",
                  "action": "Hidden"
                }
            ],
            "general": [],
            "wifi": []
        }"#;
        let mut config: Config = Config::new(&data).expect("Deserialize failed");
        assert_eq!(config.cbus.len(), 4);
        assert_eq!(config.general.len(), 0);
        assert_eq!(config.wifi.len(), 0);
    }
}
