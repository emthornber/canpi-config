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

use glob::glob;
use jsonschema::JSONSchema;
use schemars::schema::RootSchema;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::Value;

use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    string::String,
};

use backitup::backup;

use log::{error, info};

use thiserror::Error;

fn create_json_schema(root_schema: RootSchema) -> JSONSchema {
    let schema_string = serde_json::to_string(&root_schema).unwrap();
    let json_value: Value =
        serde_json::from_slice(schema_string.as_bytes()).expect("convert schema to json");
    JSONSchema::options()
        .compile(&json_value)
        .expect("A valid schema")
}

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
    /// The error was caused when reading the diagram definition file list
    #[error("cannot read diagram definition file names")]
    Glob(#[from] glob::GlobError),
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
/// Menu Item Definitions
///

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
#[allow(dead_code)]
pub struct Cfg {
    schema: JSONSchema,
    pub cfg: Option<ConfigHash>,
}

impl Cfg {
    /// Creates a new instance of the structure
    ///
    /// The type definition of ConfigHash is used to create a compiled JSON schema that will be used
    /// to validate the Attribute definitions being loaded to ConfigHash
    ///
    pub fn new<P: AsRef<Path>>(cfg_path: P, def_path: P) -> Cfg {
        let schema = Self::create_confighash_schema();
        let cfg = Self::load_configuration(cfg_path, def_path, &schema);
        Cfg { schema, cfg }
    }

    /// Create a compiled JSON schema from Attribute definition via type alias ConfigHash
    fn create_confighash_schema() -> JSONSchema {
        let attr_schema = schema_for!(ConfigHash);
        create_json_schema(attr_schema)
    }

    /// Load the attribute definitions from `def_path` and then update the current values from `cfg_path`
    fn load_configuration<P: AsRef<Path>>(
        cfg_path: P,
        def_path: P,
        schema: &JSONSchema,
    ) -> Option<ConfigHash> {
        let attr = Self::read_defn_file(def_path, &schema);
        match attr {
            Ok(defn) => Self::update_cfg_from_defn(defn, cfg_path),
            Err(e) => {
                eprintln!("{}", e);
                None
            }
        }
    }

    /// Read the contents of a file as JSON and, if valid against the schema, return an instance
    /// of 'ConfigHash'
    fn read_defn_file<P: AsRef<Path>>(
        path: P,
        schema: &JSONSchema,
    ) -> Result<ConfigHash, CfgError> {
        // Open the file in read-only mode with buffer
        let f = File::open(path.as_ref());
        match f {
            Ok(file) => {
                let reader = BufReader::new(file);

                if let Ok(json_value) = serde_json::from_reader(reader) {
                    if schema.is_valid(&json_value) {
                        // Read the JSON contents of the file as an instance of 'ConfigHash'.
                        if let Ok(cfg) = serde_json::from_value(json_value) {
                            Ok(cfg)
                        } else {
                            eprintln!("conversion to struct failed");
                            Err(CfgError::Schema(
                                "(failed to convert JSON to struct)".to_string(),
                            ))
                        }
                    } else {
                        let result = schema.validate(&json_value);
                        let pathstr = path.as_ref().to_str().unwrap();
                        if let Err(errors) = result {
                            eprintln!("schema errors");
                            for error in errors {
                                eprintln!("{}", error)
                            }
                        }
                        eprintln!("{} failed validation", pathstr);
                        Err(CfgError::Schema(pathstr.to_string()))
                    }
                } else {
                    eprintln!("reading file as json failed");
                    Err(CfgError::Schema("(non-utf8 path)".to_string()))
                }
            }
            Err(e) => Err(CfgError::Io(e)),
        }
    }

    /// Read the INI format file 'path' and create a ConfigHash from the matching entries in the
    /// definition file and update the 'current' field with value from 'path'.
    fn update_cfg_from_defn<P: AsRef<Path>>(defn: ConfigHash, path: P) -> Option<ConfigHash> {
        // Read existing configuration file
        if let Ok(ini) = Ini::load_from_file(path) {
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
                    info!("Key '{}' not defined in configuration", k);
                }
            }
            Some(cfg)
        } else {
            Some(defn)
        }
    }

    /// Get the attribute definition for the configuration item defined by `key`
    pub fn read_attribute(&self, key: String) -> Option<&Attribute> {
        match &self.cfg {
            Some(c) => {
                let attr = c.get(&key);
                match attr {
                    Some(a) => Some(a),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Store an updated attribute definition for the configuration item defined by `key`
    pub fn write_attribute(&mut self, key: String, value: &Attribute) -> Result<(), CfgError> {
        let cfg = self.cfg.clone();
        if let Some(mut c) = cfg {
            c.insert(key.to_string(), value.clone());
            self.cfg = Some(c);
            return Ok(());
        }
        Err(CfgError::Cfg())
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
}

#[cfg(test)]
mod test_cfg {
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
    #[ignore = "verbose output"]
    fn view_generated_schema() {
        let attr_schema = schema_for!(ConfigHash);
        println!("{}", serde_json::to_string_pretty(&attr_schema).unwrap());
    }

    #[test]
    #[should_panic]
    fn read_defn_file_missing() {
        let schema = Cfg::create_confighash_schema();
        let json_file = "tests/nonexistent_file.json";
        let _p = Cfg::read_defn_file(json_file, &schema).unwrap();
    }

    #[test]
    fn read_defn_file_not_valid() {
        let defn_file = "scratch/single_malformed_vector.json";
        setup_file(&defn_file, BAD_DATA);
        let schema = Cfg::create_confighash_schema();
        let bad_result = Cfg::read_defn_file(&defn_file, &schema);
        teardown_file(&defn_file);
        match bad_result {
            Ok(_) => assert!(false),
            Err(e) => {
                eprintln!("{}", e);
                assert!(true);
            }
        }
    }

    #[test]
    /// Test creating a ConfigHash
    fn read_defn_file_validates() {
        let defn_file = "scratch/single_good_vector.json";
        setup_file(&defn_file, DEFN_DATA);
        let schema = Cfg::create_confighash_schema();
        let good_result = Cfg::read_defn_file(&defn_file, &schema);
        teardown_file(&defn_file);
        match good_result {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false);
            }
        }
    }

    #[test]
    /// Test the updating of current values from the .cfg file
    fn update_with_cfg_test() {
        let cfg_file = "scratch/update_test.cfg";
        let defn_file = "scratch/update_test.json";
        setup_file(&defn_file, DEFN_DATA);
        setup_file(&cfg_file, CFG_DATA);
        let cfg = Cfg::new(&cfg_file, &defn_file);
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
    fn load_configuration_test() {
        dotenv().ok();
        let cfg_file = env::var("CFG_FILE").expect("CFG_FILE is not set in .env file");
        let def_file = env::var("DEF_FILE").expect("DEF_FILE is not set in .env file");

        let schema = Cfg::create_confighash_schema();
        let config_hash = Cfg::load_configuration(cfg_file, def_file, &schema);

        match config_hash {
            Some(ch) => {
                let attr = ch.get("router_ssid");
                if let Some(a) = attr {
                    assert_eq!(a.current, "home");
                } else {
                    assert!(false);
                }
            }
            None => {
                eprintln!("Config Hash not created");
                assert!(false);
            }
        }
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
#[allow(dead_code)]
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
    pub fn new<P: AsRef<Path>>(def_path: P) -> Pkg {
        let schema = Self::create_packagehash_schema();
        let packages = Self::load_packages(def_path, &schema);
        Pkg { schema, packages }
    }

    /// Create a compiled JSON schema from Package definition
    /// via type alias PackageHash
    fn create_packagehash_schema() -> JSONSchema {
        let src_schema = schema_for!(PackageHash);
        create_json_schema(src_schema)
    }

    /// Load the package definitions from `def_path`
    fn load_packages<P: AsRef<Path>>(def_path: P, schema: &JSONSchema) -> Option<PackageHash> {
        // Read JSON file
        let pkg = Self::read_defn_file(def_path, &schema);
        match pkg {
            Ok(packages) => {
                if packages.len() > 0 {
                    Some(packages)
                } else {
                    None
                }
            }
            Err(e) => {
                //log error text
                eprintln!("{}", e);
                None
            }
        }
    }

    /// Read the contents of a file as JSON and, if valid against the schema, return an instance
    /// of 'PackageHash'
    fn read_defn_file<P: AsRef<Path>>(
        path: P,
        schema: &JSONSchema,
    ) -> Result<PackageHash, CfgError> {
        // Open the file in read-only mode with buffer
        let f = File::open(path.as_ref());
        match f {
            Ok(file) => {
                let reader = BufReader::new(file);

                if let Ok(json_value) = serde_json::from_reader(reader) {
                    if schema.is_valid(&json_value) {
                        // Read the JSON contents of the file as an instance of 'PackageHash'.
                        if let Ok(pkg) = serde_json::from_value(json_value) {
                            Ok(pkg)
                        } else {
                            eprintln!("conversion to struct failed");
                            Err(CfgError::Schema(
                                "(failed to convert JSON to struct)".to_string(),
                            ))
                        }
                    } else {
                        let result = schema.validate(&json_value);
                        let pathstr = path.as_ref().to_str().unwrap();
                        if let Err(errors) = result {
                            eprintln!("schema errors");
                            for error in errors {
                                eprintln!("{}", error);
                            }
                        }
                        eprintln!("{} failed validation", pathstr);
                        Err(CfgError::Schema(pathstr.to_string()))
                    }
                } else {
                    eprintln!("reading file as json failed");
                    Err(CfgError::Schema("(non-utf8 path)".to_string()))
                }
            }
            Err(e) => Err(CfgError::Io(e)),
        }
    }
}

#[cfg(test)]
mod test_pkg {
    use super::*;

    #[test]
    #[ignore = "verbose output"]
    fn view_pkg_schema() {
        let pkg_schema = schema_for!(PackageHash);
        println!("{}", serde_json::to_string_pretty(&pkg_schema).unwrap())
    }

    #[test]
    #[should_panic]
    fn read_defn_file_missing() {
        let schema = Pkg::create_packagehash_schema();
        let json_file = "tests/nonexistent_file.json";
        let _p = Pkg::read_defn_file(json_file, &schema).unwrap();
    }

    #[test]
    #[should_panic]
    fn read_defn_file_not_valid() {
        let schema = Pkg::create_packagehash_schema();
        let json_file = "tests/good-example-config-defn.json";
        let _p = Pkg::read_defn_file(json_file, &schema).unwrap();
    }

    #[test]
    fn read_defn_file_validates() {
        let schema = Pkg::create_packagehash_schema();
        let json_file = "tests/test_pkg.json";
        let p = Pkg::read_defn_file(json_file, &schema).unwrap();
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn load_pkg_no_json() {
        let schema = Pkg::create_packagehash_schema();
        let def_path = "src/";
        let ph = Pkg::load_packages(def_path, &schema);
        match ph {
            Some(_) => assert!(false),
            None => assert!(true),
        }
    }

    #[test]
    fn load_pkg_invalid_json() {
        let schema = Pkg::create_packagehash_schema();
        let def_path = "tests/bad-example-config-defn.json";
        let ph = Pkg::load_packages(def_path, &schema);
        match ph {
            Some(_) => assert!(false),
            None => assert!(true),
        }
    }

    #[test]
    fn load_pkg_valid_json() {
        let schema = Pkg::create_packagehash_schema();
        let def_path = "tests/test_pkg.json";
        let ph = Pkg::load_packages(def_path, &schema);
        match ph {
            Some(ph) => assert_eq!(ph.len(), 2),
            None => assert!(false),
        }
    }
}
///
/// Signalling Panel Definitions
///
/// The enum and struct definitions are detailed so that JSON diagram definition
/// can be parsed successfully.
/// The only attribute used is diagram.layout.panel.title hence the supression
/// of dead code warnings.

/// Enumerations
///
/// Direction of track marking on a Tile
#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq)]
#[allow(dead_code)]
pub enum Direction {
    EW,
    NE,
    NS,
    NW,
    SE,
    SW,
}

/// State of CBus event
#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq)]
#[allow(dead_code)]
pub enum State {
    UNKN,
    ZERO,
    ONE,
}

/// Type of control switch
#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq)]
pub enum SwitchType {
    Toggle,
    PushButton,
}

#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq)]
#[allow(dead_code)]
pub enum TurnOutDirection {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq)]
#[allow(dead_code)]
pub enum TurnOutHand {
    Left,
    Right,
    Wye,
}

/// Structures
///
/// CbusStates that indicate how the turnout is lying
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct TurnoutState {
    /// Treat these as a double-bit
    /// 0-0 In-transit
    /// 1-0 Normal
    /// 0-1 Reverse
    /// 1-1 ERROR
    normal: String,
    reverse: String,
}

/// Definition of the state of a CBus event
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct CbusState {
    /// Item name
    name: String,
    /// Event number - either long or short format
    event: String,
    /// Current state of the event
    state: State,
}

/// Dimensions of the panel
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Panel {
    /// Width of panel in tiles
    width: u16,
    /// Height of panel in tiles
    height: u16,
    /// size (in pixels) of a square tile
    tilesize: u16,
    /// RGB colour definition of panel background as a HEX string
    colour: String,
    /// Margin in pixels
    margins: u16,
    //// Border in pixels
    border: u16,
    /// Diagram title
    title: String,
}

/// Position of tile within panel
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Tile {
    /// (1 <= x_coord <= panel.width)
    x_coord: u16,
    /// (1 <= y_coord <= panel.height)
    y_coord: u16,
}

// How the track is shown on a tile
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Track {
    /// Where the track is on the panel
    tile: Tile,
    /// Which image to use
    direction: Direction,
    /// Text to be displayed on panel
    label: Option<String>,
    /// CbusState that provides state of track circuit
    tcstate: Option<String>,
    /// CbusState that provides state of train detector
    spot: Option<String>,
}

/// Turnout (switch, point) details
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Turnout {
    /// Where on panel
    tile: Tile,
    /// Text to be displayed on panel
    name: String,
    /// Left, Right of Wye
    hand: TurnOutHand,
    /// Direction turnout is laid
    orientation: TurnOutDirection,
    /// CbusStates that define the turnout state
    tostate: TurnoutState,
}

/// Definition of a control switch
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Control {
    /// Position of switch on panel
    tile: Tile,
    /// Display name
    name: String,
    switch: SwitchType,
    /// Name of CBusState that actuates turnout
    action: String,
    /// How the turnout currently lies
    tostate: TurnoutState,
}

/// Specification of the signalling diagram
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Layout {
    /// Overall panel details
    panel: Panel,
    /// List of controls for turnouts, signals, ...
    controls: Vec<Control>,
    /// Track layout
    track: Vec<Track>,
    /// Turnout definitions
    turnouts: Vec<Turnout>,
}

/// Definition of a Signalling Panel
#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct Diagram {
    /// The state of the CBus producers and consumers
    cbusstates: Vec<CbusState>,
    /// The realisation of the signalling diagram
    layout: Layout,
}

#[derive(Clone, Deserialize, Debug, JsonSchema)]
#[allow(dead_code)]
pub struct PanelDefinition {
    title: String,
    json_file: PathBuf,
}

/// Type alias defining the signalling diagram JSON files
pub type PanelHash = HashMap<u8, PanelDefinition>;

/// The definition of available control panels
#[allow(dead_code)]
pub struct PanelList {
    schema: JSONSchema,
    pub panels: Option<PanelHash>,
}

impl PanelList {
    /// Create a new instance of the structure
    ///
    /// The type definition of Diagram is used to create
    /// a compiled JSON schema that will be used to validate
    /// the panel definition being referenced by PanelHash.
    pub fn new<P: AsRef<Path>>(panel_dir: P) -> PanelList {
        let schema = Self::create_diagram_schema();
        let panels = Self::load_panels(panel_dir, &schema);
        PanelList { schema, panels }
    }

    /// Create a compiled JSON schema from Diagram definition
    fn create_diagram_schema() -> JSONSchema {
        let schema = schema_for!(Diagram);
        create_json_schema(schema)
    }

    /// Load the panel definitions from `${panel_dir}/*.json`, extract 'title'
    /// from JSON definitions, and store in PanelHash.
    fn load_panels<P: AsRef<Path>>(panel_dir: P, schema: &JSONSchema) -> Option<PanelHash> {
        // Read list of JSON files
        let pf = Self::find_panel_definitions(panel_dir);
        match pf {
            Ok(panel_files) => {
                let mut index = 1;
                let mut panels = PanelHash::new();
                // Walk through file list
                for panel in panel_files {
                    if let Ok(panel_defn) = Self::read_defn_file(panel, schema) {
                        // JSON file validated successfully so add to PanelHash
                        if let Some(_) = panels.insert(index, panel_defn) {
                            index += 1;
                        }
                    }
                    // ignore failures
                }
                if panels.len() > 0 {
                    Some(panels)
                } else {
                    None
                }
            }
            Err(e) => {
                // Log error text
                eprintln!("{}", e);
                None
            }
        }
    }

    /// Return list of all JSON files in 'panels' directory
    fn find_panel_definitions<P: AsRef<Path>>(panel_dir: P) -> Result<Vec<PathBuf>, CfgError> {
        let mut panel_vec: Vec<PathBuf> = Vec::new();
        let mut panel_json = panel_dir.as_ref().to_path_buf();
        panel_json.push("*.json");
        if let Some(glob_str) = panel_json.to_str() {
            for entry in glob(glob_str).unwrap().filter_map(Result::ok) {
                panel_vec.push(entry);
            }
        }
        Ok(panel_vec)
    }

    /// Read the contents of a file as JSON and, if valid against the schema,
    /// return an instance of 'PanelDefinition'
    fn read_defn_file<P: AsRef<Path>>(
        path: P,
        schema: &JSONSchema,
    ) -> Result<PanelDefinition, CfgError> {
        // Open the file in read-only mode with buffer
        let f = File::open(path.as_ref());
        match f {
            Ok(file) => {
                let reader = BufReader::new(file);
                if let Ok(json_value) = serde_json::from_reader(reader) {
                    if schema.is_valid(&json_value) {
                        // Read the JSON contents of the file as an instance of 'Diagram'.
                        if let Ok(diagram) = serde_json::from_value::<Diagram>(json_value) {
                            let title = diagram.layout.panel.title;
                            let json_file = path.as_ref().to_path_buf();
                            let panel_entry = PanelDefinition { title, json_file };
                            Ok(panel_entry)
                        } else {
                            eprintln!("conversion to struct failed");
                            Err(CfgError::Schema(
                                "(failed to convert JSON to struct)".to_string(),
                            ))
                        }
                    } else {
                        // JSON not valid against schema - log detailed error report
                        let result = schema.validate(&json_value);
                        let pathstr = path.as_ref().to_str().unwrap();
                        if let Err(errors) = result {
                            eprintln!("schema errors");
                            for error in errors {
                                eprintln!("{}", error);
                            }
                        }
                        eprintln!("{} failed validation", pathstr);
                        Err(CfgError::Schema(pathstr.to_string()))
                    }
                } else {
                    eprintln!("reading file as json failed");
                    Err(CfgError::Schema("(non-utf8 path)".to_string()))
                }
            }
            Err(e) => Err(CfgError::Io(e)),
        }
    }
}

#[cfg(test)]
mod test_panel_list {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn setup_file<P: AsRef<Path>>(test_file: P, data: &str) {
        let mut f = File::create(test_file).expect("file creation failed");
        f.write_all(data.as_bytes()).expect("file write failed");
    }

    fn teardown_file<P: AsRef<Path>>(test_file: P) {
        fs::remove_file(test_file).expect("file deletion failed");
    }

    #[test]
    #[ignore = "verbose output"]
    fn view_diagram_schema() {
        let dia_schema = schema_for!(Diagram);
        println!("{}", serde_json::to_string_pretty(&dia_schema).unwrap());
    }

    #[test]
    fn find_panel_definitions_zero() {
        let panel_dir = "src/";
        let pf = PanelList::find_panel_definitions(&panel_dir).unwrap();
        assert_eq!(pf.len(), 0);
    }

    #[test]
    fn find_panel_definitions_more_than_zero() {
        let json_file = "scratch/panel.json";
        setup_file(json_file, "{}");
        let panel_dir = "scratch/";
        let pf = PanelList::find_panel_definitions(&panel_dir).unwrap();
        assert!(pf.len() > 0);
        teardown_file(json_file)
    }

    #[test]
    #[should_panic]
    fn read_defn_file_missing() {
        let schema = PanelList::create_diagram_schema();
        let json_file = "tests/nonexistent_file.json";
        let _pd = PanelList::read_defn_file(json_file, &schema).unwrap();
    }

    #[test]
    #[should_panic]
    fn read_defn_file_not_valid() {
        let schema = PanelList::create_diagram_schema();
        let json_file = "tests/good-example-config-defn.json";
        let _pd = PanelList::read_defn_file(json_file, &schema).unwrap();
    }

    #[test]
    fn read_defn_file_validates() {
        let schema = PanelList::create_diagram_schema();
        let json_file = "tests/test_diagram.json";
        let pd = PanelList::read_defn_file(json_file, &schema).unwrap();
        assert_eq!(pd.title, "Test Diagram");
    }

    #[test]
    fn load_panels_no_json() {
        let schema = PanelList::create_diagram_schema();
        let panel_dir = "src/";
        let panel_hash = PanelList::load_panels(panel_dir, &schema);
        match panel_hash {
            Some(_) => assert!(false),
            None => assert!(true),
        }
    }

    #[test]
    fn load_panels_invalid_json() {
        let schema = PanelList::create_diagram_schema();
        let panel_dir = "scratch/";
        let panel_hash = PanelList::load_panels(panel_dir, &schema);
        match panel_hash {
            Some(_) => assert!(false),
            None => assert!(true),
        }
    }

    #[test]
    fn load_panels_one_valid_json() {
        let schema = PanelList::create_diagram_schema();
        let panel_dir = "tests/";
        let panel_hash = PanelList::load_panels(panel_dir, &schema);
        match panel_hash {
            Some(ph) => assert_eq!(ph.len(), 1),
            None => assert!(false),
        }
    }
}
