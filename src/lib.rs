//! # canpi-config
//!
//! A crate to provide functionality to read and write the canpi server configuration
//! and to define which configuration items can be changed or viewed by the user and which are hidden.
//!
//! There is a JSON file that defines the configuration item format and default values
//! along with a matching schema file.  This file is loaded to the ConfigHash.  The canpi INI file,
//! if it exists, is read to update current value of the configuration items so the ConfigHash
//! becomes the single source of truth.
//!
//! There is a function to write the ConfigHash current values as an INI file.
//
//  30 November, 2021 - E M Thornber
//

use ini::Ini;

use jsonschema::JSONSchema;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::Value;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::string::String;

use backitup::backup;

use thiserror::Error;

#[derive(Error, Debug)]
/// Categorizes the cause of errors when processing the configuration files
pub enum CfgError {
    /// The error was caused by a failure to read the configuration file
    #[error("cannot open configuration file")]
    Io(#[from] std::io::Error),
    /// The error was caused by failure to validate JSON input
    #[error("JSON input '{0}' failed to validate against schema")]
    Schema(String),
    /// The error was caused by a failure to deserialize the JSON
    #[error("cannot deserialize configuration file")]
    Json(#[from] serde_json::Error),
    /// The error was caused when reading or writing the .cfg file
    #[error("cannot read/write cfg file")]
    Ini(#[from] ini::Error),
    /// The error was caused by a lack of attribute definitions
    #[error("Cfg structure not properly initialised")]
    Cfg(),
}

impl std::convert::From<jsonschema::SchemaResolverError> for CfgError {
    fn from(err: jsonschema::SchemaResolverError) -> Self {
        CfgError::Schema(err.to_string())
    }
}

///
/// Attribute Definitions
///
#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq)]
/// Defines the possible behaviours for an attribute
pub enum ActionBehaviour {
    /// User can update the value of current field
    Edit,
    /// User can see the value of the current field but cannot change it
    Display,
    /// Attribute is for internal use only
    Hide,
}

#[derive(Clone, Deserialize, Debug, JsonSchema)]
/// Definition of an attribute
pub struct Attribute {
    /// Text used to label edit box on form
    pub prompt: String,
    /// Text displayed when the user hovers over edit box
    pub tooltip: String,
    /// Current value of attribute.  Used to populate .cfg file
    pub current: String,
    /// Default value of attribute
    pub default: String,
    /// Regular expression to validate user input
    pub format: String,
    /// How the attribute is presented on a webpage
    pub action: ActionBehaviour,
}

/// Type alias based on a HashMap
pub type ConfigHash = HashMap<String, Attribute>;

/// The structure that holds the definition of configuration items
pub struct Cfg {
    schema: JSONSchema,
    cfg: Option<ConfigHash>,
}

impl Cfg {
    /// Creates a new instance of the structure
    ///
    /// The type definition of ConfigHash is used to create a compiled JSON schema that will be used
    /// to validate the Attribute definitions being loaded to ConfigHash
    ///
    /// Note: load_configuration must be called to fully initialise the structure
    pub fn new() -> Cfg {
        let schema = Self::create_defn_schema();
        Cfg {
            schema: schema,
            cfg: None,
        }
    }

    /// Load the attribute definitions from `def_path` and then update the current values from `cfg_path`
    pub fn load_configuration<P: AsRef<Path>>(
        &mut self,
        cfg_path: P,
        def_path: P,
    ) -> Result<(), CfgError> {
        let defn = Self::read_defn_file(def_path, &self.schema)?;
        self.update_cfg_from_defn(defn, cfg_path)?;

        Ok(())
    }

    /// Get the attribute definition for the configuration item defined by `key`
    pub fn read_attribute(&self, key: String) -> Option<&Attribute> {
        match &self.cfg {
            Some(c) => {
                let attr = c.get(&key);
                match attr {
                    Some(a) => Some(a).clone(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Store an updated attribute definition for the configuration item defined by `key`
    pub fn write_attribute(&mut self, key: String, value: &Attribute) -> Result<(), CfgError> {
        let cfg = self.cfg.clone();
        match cfg {
            Some(mut c) => {
                c.insert(key.to_string(), value.clone());
                self.cfg = Some(c);
                return Ok(());
            }
            _ => {}
        }
        Err(CfgError::Cfg())
    }

    /// Create a compiled JSON schema from Attribute definition via type alias ConfigHash
    fn create_defn_schema() -> JSONSchema {
        let attr_schema = schema_for!(ConfigHash);
        //println!("{}", serde_json::to_string_pretty(&attr_schema).unwrap());
        let schema_string = serde_json::to_string(&attr_schema).unwrap();
        let json_value: Value =
            serde_json::from_slice(schema_string.as_bytes()).expect("convert schema to json");
        JSONSchema::options()
            .compile(&json_value)
            .expect("A valid schema")
    }

    /// Read the contents of a file as JSON and, if valid against the schema, return an instance
    /// of 'ConfigHash'
    fn read_defn_file<P: AsRef<Path>>(
        path: P,
        schema: &JSONSchema,
    ) -> Result<ConfigHash, CfgError> {
        // Open the file in read-only mode with buffer
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);

        let json_value: Value = serde_json::from_reader(reader)?;
        if schema.is_valid(&json_value) {
            // Read the JSON contents of the file as an instance of 'ConfigHash'.
            let cfg = serde_json::from_value(json_value)?;
            return Ok(cfg);
        }
        if let Some(f) = path.as_ref().to_str() {
            return Err(CfgError::Schema(f.to_string()));
        }
        Err(CfgError::Schema("(non-utf8 path".to_string()))
    }

    /// Filters the attributes by action
    pub fn attributes_with_action(&self, action: ActionBehaviour) -> ConfigHash {
        let mut attr2 = ConfigHash::new();
        if let Some(cfg) = self.cfg.clone() {
            attr2.extend(
                cfg.iter()
                    .filter(|(_k, v)| v.action == action)
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
        }
        attr2
    }

    /// Output the keys and current values of items to `path`
    ///
    /// If makeBackup is TRUE then a timestamped backup of the existing INI file is taken
    ///
    /// Note: The format of the output file is INI with just a general section
    pub fn write_cfg_file<P: AsRef<Path>>(
        &self,
        path: P,
        make_backup: Option<bool>,
    ) -> Result<(), CfgError> {
        let c = &self.cfg;
        if let Some(cfg) = c {
            let mut ini = Ini::new();
            for (k, v) in cfg {
                ini.set_to(None::<String>, k.clone(), v.current.clone());
            }
            let mut do_backup: bool = false;
            if let Some(b) = make_backup {
                do_backup = b;
            }
            if do_backup {
                match backup(&path) {
                    Ok(backup_path) => println!("Backup created: {:?}", backup_path),
                    Err(err) => eprintln!("Failed to create backup: {:?}", err),
                }
            }
            ini.write_to_file(path)?;
        }
        Ok(())
    }

    /// Read the INI format file 'path' and create a ConfigHash from the matching entries in the
    /// definition file and update the 'current' field with value from 'path'.
    fn update_cfg_from_defn<P: AsRef<Path>>(
        &mut self,
        defn: ConfigHash,
        path: P,
    ) -> Result<(), CfgError> {
        // Read existing configuration file
        let ini = Ini::load_from_file(path)?;
        // Create new ConfigHash to hold configuration
        let mut cfg = ConfigHash::new();
        let properties = ini.general_section();
        for (k, v) in properties.iter() {
            let attr = defn.get(k);
            if let Some(aref) = attr {
                let mut a = aref.clone();
                a.current = v.to_string();
                cfg.insert(k.to_string(), a);
            } else {
                println!("Key '{}' not defined in configuration", k);
            }
        }
        self.cfg = Some(cfg);
        Ok(())
    }
}

///
/// Package Definitions
///
#[derive(Clone, Deserialize, Debug, JsonSchema)]
/// Definition of a Package
pub struct Package {
    /// Path of package directory
    pub cfg_path: String,
    /// Name of INI file
    pub ini_file: String,
    /// Name of Attribute Definition File
    pub json_file: String,
}

/// Type alias based on a HashMap
pub type PackageHash = HashMap<String, Package>;

/// The structure that holds the definition of package items
pub struct Pkg {
    schema: JSONSchema,
    pub packages: Option<PackageHash>,
}

impl Pkg {
    /// Creates a new instance of the structure
    ///
    /// The type definition of PackageHash is used to create a compiled JSON schema that will be used
    /// to validate the Package definitions being loaded to PackageHash
    ///
    /// Note: load_packages must be called to fully initialise the structure
    pub fn new() -> Pkg {
        let schema = Self::create_defn_schema();
        Pkg {
            schema: schema,
            packages: None,
        }
    }

    /// Create a compiled JSON schema from Package definition via type alias PackageHash
    fn create_defn_schema() -> JSONSchema {
        let src_schema = schema_for!(PackageHash);
        //println!("{}", serde_json::to_string_pretty(&src_schema).unwrap());
        let schema_string = serde_json::to_string(&src_schema).unwrap();
        let json_value: Value =
            serde_json::from_slice(schema_string.as_bytes()).expect("convert schema to json");
        JSONSchema::options()
            .compile(&json_value)
            .expect("A valid schema")
    }

    /// Load the package definitions from `def_path`
    pub fn load_packages<P: AsRef<Path>>(&mut self, def_path: P) -> Result<(), CfgError> {
        let pkg = Self::read_defn_file(def_path, &self.schema)?;

        self.packages = Some(pkg);
        Ok(())
    }

    /// Read the contents of a file as JSON and, if valid against the schema, return an instance
    /// of 'PackageHash'
    fn read_defn_file<P: AsRef<Path>>(
        path: P,
        schema: &JSONSchema,
    ) -> Result<PackageHash, CfgError> {
        // Open the file in read-only mode with buffer
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);

        let json_value: Value = serde_json::from_reader(reader)?;
        if schema.is_valid(&json_value) {
            // Read the JSON contents of the file as an instance of 'PackageHash'.
            let pkg = serde_json::from_value(json_value)?;
            return Ok(pkg);
        }
        if let Some(f) = path.as_ref().to_str() {
            return Err(CfgError::Schema(f.to_string()));
        }
        Err(CfgError::Schema("(non-utf8 path".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use std::io::Write;
    use std::{env, fs};

    const CFG_DATA: &str = r#"
        canid=101
        node_number=5432
        start_event_id=2
        node_mode=1
        "#;

    const DEFN_DATA: &str = r#"
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

    const BAD_DATA: &str = r#"
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

    fn setup_file<P: AsRef<Path>>(test_file: P, data: &str) {
        let mut f = File::create(test_file).expect("file creation failed");
        f.write_all(data.as_bytes()).expect("file write failed");
    }

    fn teardown_file<P: AsRef<Path>>(test_file: P) {
        fs::remove_file(test_file).expect("file deletion failed");
    }

    #[test]
    fn single_attribute() {
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
        let a: Attribute = serde_json::from_str(data).expect("Failed to deserialize");

        // println!("Attribute is {} ({})", a.attribute, a.tooltip);
        assert_eq!(a.action, ActionBehaviour::Display);
    }

    #[test]
    /// Test creating a ConfigHash
    fn single_good_vector() {
        let defn_file = "scratch/single_good_vector.json";
        setup_file(&defn_file, DEFN_DATA);
        let schema = Cfg::create_defn_schema();
        Cfg::read_defn_file(&defn_file, &schema).expect("parameter definition failed to load");
        teardown_file(&defn_file);
    }

    #[test]
    #[should_panic]
    fn single_malformed_vector() {
        let defn_file = "scratch/single_malformed_vector.json";
        setup_file(&defn_file, BAD_DATA);
        let schema = Cfg::create_defn_schema();
        Cfg::read_defn_file(&defn_file, &schema).expect("parameter definition failed to load");
    }

    #[test]
    /// Test the updating of current values from the .cfg file
    fn update_with_cfg_test() {
        let cfg_file = "scratch/update_test.cfg";
        let defn_file = "scratch/update_test.json";
        setup_file(&defn_file, DEFN_DATA);
        setup_file(&cfg_file, CFG_DATA);
        let mut cfg = Cfg::new();
        cfg.load_configuration(&cfg_file, &defn_file)
            .expect("parameter definition failed to load");
        let ini = Ini::load_from_file(&cfg_file).expect("failed to load .cfg file");
        if let Some(config) = cfg.cfg.clone() {
            let properties = ini.section(None::<String>);
            if let Some(p) = properties {
                for (k, v) in p.iter() {
                    let attr = config.get(k);
                    if let Some(a) = attr {
                        assert_eq!(a.current, v.to_string(), "attribute {} not updated", k);
                    } else {
                        assert!(false, "attribute {} missing", k);
                    }
                }
            }
        } else {
            assert!(false, "Cfg.cfg is 'None'");
        }
        teardown_file(&cfg_file);
        teardown_file(&defn_file);
    }

    #[test]
    /// Test filtering of attributes by action value via attributes_with_action()
    fn attributes_with_action_test() {
        let cfg_file = "scratch/attributes_test.cfg";
        let defn_file = "scratch/attributes_test.json";
        setup_file(&defn_file, DEFN_DATA);
        setup_file(&cfg_file, CFG_DATA);
        let mut cfg = Cfg::new();
        cfg.load_configuration(&cfg_file, &defn_file)
            .expect("config failed to load");
        if let Some(config) = cfg.cfg.clone() {
            assert_eq!(config.len(), 4);
            let displayable: ConfigHash = cfg.attributes_with_action(ActionBehaviour::Display);
            assert_eq!(displayable.len(), 2);
            assert!(displayable.contains_key("canid"));
            assert!(displayable.contains_key("node_number"));
            let editable: ConfigHash = cfg.attributes_with_action(ActionBehaviour::Edit);
            assert_eq!(editable.len(), 1);
            assert!(editable.contains_key("start_event_id"));
            let hidden: ConfigHash = cfg.attributes_with_action(ActionBehaviour::Hide);
            assert_eq!(hidden.len(), 1);
            assert!(hidden.contains_key("node_mode"));
        } else {
            assert!(false)
        }
        teardown_file(&cfg_file);
        teardown_file(&defn_file);
    }

    #[test]
    fn view_generated_schema() {
        let attr_schema = schema_for!(ConfigHash);
        println!("{}", serde_json::to_string_pretty(&attr_schema).unwrap());
    }

    #[test]
    fn write_ini_file() {
        dotenv().ok();
        let mut cfg = Cfg::new();
        let mut cfg_file = env::var("CFG_FILE").expect("CFG_FILE is not set in .env file");
        let def_file = env::var("DEF_FILE").expect("DEF_FILE is not set in .env file");
        cfg.load_configuration(cfg_file.clone(), def_file)
            .expect("config hash populated");
        cfg_file.push_str(".new");
        cfg.write_cfg_file(cfg_file, Some(true))
            .expect("Failed to write cfg file");
    }
}
