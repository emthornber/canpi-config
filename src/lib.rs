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

use schemars::Schema;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::Value;

use std::{collections::HashMap, fs::File, io::BufReader, path::Path, string::String};

use backitup::backup;

use log::{error, info};

use slugrs::slugify;
use thiserror::Error;

fn create_json_schema(schema: Schema) -> Value {
    let schema_string = serde_json::to_string(&schema).unwrap();
    let json_value: Value =
        serde_json::from_slice(schema_string.as_bytes()).expect("convert schema to json");
    json_value
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

impl std::convert::From<jsonschema::ReferencingError> for CfgError {
    fn from(err: jsonschema::ReferencingError) -> Self {
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
pub type IniHash = HashMap<String, String>;

/// The structure that holds the definition of configuration items
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Cfg {
    schema: Value,
    pub cfg: Option<ConfigHash>,
}

impl Cfg {
    /// Creates a new instance of the structure
    ///
    /// The type definition of ConfigHash is used to create a compiled JSON schema that will be used
    /// to validate the Attribute definitions being loaded to ConfigHash
    ///
    pub fn new<P: AsRef<Path> + std::fmt::Display>(cfg_path: P, def_path: P) -> Cfg {
        let schema = Self::create_confighash_schema();
        let cfg = Self::load_configuration(cfg_path, def_path, &schema);
        Cfg { schema, cfg }
    }

    /// Create a compiled JSON schema from Attribute definition via type alias ConfigHash
    fn create_confighash_schema() -> Value {
        let attr_schema = schema_for!(ConfigHash);
        create_json_schema(attr_schema)
    }

    /// Load the attribute definitions from `def_path` and then update the current values from `cfg_path`
    fn load_configuration<P: AsRef<Path> + std::fmt::Display>(
        cfg_path: P,
        def_path: P,
        schema: &Value,
    ) -> Option<ConfigHash> {
        let attr = Self::read_defn_file(def_path, &schema);
        match attr {
            Ok(defn) => Self::update_cfg_from_defn(defn, cfg_path),
            Err(e) => {
                error!("{}", e);
                None
            }
        }
    }

    /// Read the contents of a file as JSON and, if valid against the schema, return an instance
    /// of 'ConfigHash'
    fn read_defn_file<P: AsRef<Path> + std::fmt::Display>(
        path: P,
        schema: &Value,
    ) -> Result<ConfigHash, CfgError> {
        // Open the file in read-only mode with buffer
        let f = File::open(path.as_ref());
        match f {
            Ok(file) => {
                let reader = BufReader::new(file);

                if let Ok(json_value) = serde_json::from_reader(reader) {
                    if jsonschema::is_valid(schema, &json_value) {
                        // Read the JSON contents of the file as an instance of 'ConfigHash'.
                        if let Ok(cfg) = serde_json::from_value(json_value) {
                            Ok(cfg)
                        } else {
                            error!("conversion to struct failed for {}", path);
                            Err(CfgError::Schema(
                                "(failed to convert JSON to struct)".to_string(),
                            ))
                        }
                    } else {
                        let pathstr = path.as_ref().to_str().unwrap();
                        if let Ok(validator) = jsonschema::validator_for(schema) {
                            let result = validator.iter_errors(&json_value);
                            for error in result {
                                error!("{}", error)
                            }
                            error!("{} failed validation", pathstr);
                        }
                        Err(CfgError::Schema(pathstr.to_string()))
                    }
                } else {
                    error!("reading file {} as json failed", path);
                    Err(CfgError::Schema("(non-utf8 path)".to_string()))
                }
            }
            Err(e) => Err(CfgError::Io(e)),
        }
    }

    /// Read the INI format file 'path' and create a ConfigHash from the matching entries in the
    /// definition file and update the 'current' field with value from 'path'.
    fn update_cfg_from_defn<P: AsRef<Path> + std::fmt::Display>(
        defn: ConfigHash,
        path: P,
    ) -> Option<ConfigHash> {
        // Read existing configuration file
        if let Ok(ini) = Self::read_cfg_file(path) {
            // Create new ConfigHash to hold configuration
            let mut cfg = ConfigHash::new();
            for (k, v) in ini.iter() {
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

    /// Read the configuration file at `path` and populate a IniHash with the
    /// Properties from the general section
    pub fn read_cfg_file<P: AsRef<Path> + std::fmt::Display>(path: P) -> Result<IniHash, CfgError> {
        // Read existing configuration file
        let f = Ini::load_from_file(&path);
        match f {
            Ok(ini) => {
                // create new IniHash to hold .ini file contents
                let mut cfg = IniHash::new();
                // Get the general section properties
                let properties = ini.general_section();
                for (k, v) in properties.iter() {
                    cfg.insert(k.to_string(), v.to_string());
                }
                // Return the IniHash
                Ok(cfg)
            }
            Err(e) => {
                error!("reading file {} as .ini failed", path);
                Err(CfgError::Ini(e))
            }
        }
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
                    Ok(backup_path) => info!("Backup created: {:?}", backup_path),
                    Err(err) => error!("Failed to create backup: {:?}", err),
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
    use env_logger::Target;
    use log::{error, info, LevelFilter};
    use std::io::Write;
    use std::{env, fs};

    const CFG_DATA: &str = r#"
        canid=101
        node_number=5432
        start_event_id=2
        node_mode=1
        "#;

    const BAD_CFG_DATA: &str = r#"
        canid=101,
        node_number=
        start_event_id==2
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

    fn init_logging() {
        let _ = env_logger::builder()
            .target(Target::Stdout)
            .filter_level(LevelFilter::max())
            .is_test(true)
            .try_init();
    }

    fn setup_file<P: AsRef<Path> + std::fmt::Display>(test_file: P, data: &str) {
        if let Ok(mut f) = File::create(&test_file) {
            if let Err(e) = f.write_all(data.as_bytes()) {
                error!("{}: file {} write failed", e, test_file);
            }
        } else {
            error!("file {} creation failed", test_file);
        }
    }

    fn teardown_file<P: AsRef<Path> + std::fmt::Display>(test_file: P) {
        if let Err(e) = fs::remove_file(&test_file) {
            error!("{}: file {} deletion failed", e, test_file);
        }
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

        // Initialise Logger
        init_logging();

        // Parse the string of data into an Attribute object.
        let a: Result<Attribute, serde_json::Error> = serde_json::from_str(data);
        match a {
            Ok(a) => {
                info!("Attribute is {} ({})", a.prompt, a.tooltip);
                assert_eq!(a.action, ActionBehaviour::Display);
            }
            Err(e) => error!("{}: Failed to deserialize", e),
        }
    }

    #[test]
    #[ignore = "verbose output"]
    fn view_generated_schema() {
        // Initialise Logger
        init_logging();

        let attr_schema = schema_for!(ConfigHash);
        info!("{}", serde_json::to_string_pretty(&attr_schema).unwrap());
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
        // Initialise Logger
        init_logging();

        let defn_file = "scratch/single_malformed_vector.json";
        setup_file(&defn_file, BAD_DATA);
        let schema = Cfg::create_confighash_schema();
        let bad_result = Cfg::read_defn_file(&defn_file, &schema);
        teardown_file(&defn_file);
        match bad_result {
            Ok(_) => assert!(false),
            Err(e) => {
                error!("{}", e);
                assert!(true);
            }
        }
    }

    #[test]
    /// Test creating a ConfigHash
    fn read_defn_file_validates() {
        // Initialise Logger
        init_logging();

        let defn_file = "scratch/single_good_vector.json";
        setup_file(&defn_file, DEFN_DATA);
        let schema = Cfg::create_confighash_schema();
        let good_result = Cfg::read_defn_file(&defn_file, &schema);
        teardown_file(&defn_file);
        match good_result {
            Ok(_) => assert!(true),
            Err(e) => {
                error!("{}", e);
                assert!(false);
            }
        }
    }

    #[test]
    #[should_panic]
    fn read_cfg_file_missing() {
        let cfg_file = "tests/nonexistent_file.ini";
        let _p = Cfg::read_cfg_file(cfg_file).unwrap();
    }

    #[test]
    #[should_panic]
    fn read_cfg_file_not_valid() {
        // Initialise Logger
        init_logging();

        let cfg_file = "scratch/malformed_cfg_file.ini";
        setup_file(&cfg_file, BAD_CFG_DATA);
        let bad_result = Cfg::read_cfg_file(&cfg_file);
        teardown_file(&cfg_file);
        match bad_result {
            Ok(_) => assert!(false),
            Err(e) => {
                error!("{}", e);
                assert!(true);
            }
        }
    }

    #[test]
    fn read_cfg_file_validates() {
        // Initialise Logger
        init_logging();

        let cfg_file = "scratch/good_cfg_file.ini";
        setup_file(&cfg_file, CFG_DATA);
        let good_result = Cfg::read_cfg_file(&cfg_file);
        teardown_file(&cfg_file);
        match good_result {
            Ok(gr) => {
                if gr.len() == 4 {
                    assert!(true)
                } else {
                    assert!(
                        false,
                        "read_cfg_file_validates: expected 4 items, got {}",
                        gr.len()
                    );
                }
            }
            Err(e) => {
                error!("{}", e);
                assert!(false);
            }
        }
    }

    #[test]
    /// Test the updating of current values from the .cfg file
    fn update_with_cfg_test() {
        // Initialise Logger
        init_logging();

        let cfg_file = "scratch/update_test.cfg";
        let defn_file = "scratch/update_test.json";
        setup_file(&defn_file, DEFN_DATA);
        setup_file(&cfg_file, CFG_DATA);
        let cfg = Cfg::new(&cfg_file, &defn_file);
        let ini = Ini::load_from_file(&cfg_file);
        teardown_file(&cfg_file);
        teardown_file(&defn_file);
        match ini {
            Ok(ini) => {
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
            }
            Err(_) => assert!(false, "failed to load .cfg file"),
        }
    }

    #[test]
    fn load_configuration_test() {
        // Initialise Logger
        init_logging();

        dotenv().ok();
        if let Ok(cfg_file) = env::var("CFG_FILE") {
            if let Ok(def_file) = env::var("DEF_FILE") {
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
                    None => assert!(false, "Config Hash not created"),
                }
            } else {
                assert!(false, "DEF_FILE is not set in .env file");
            }
        } else {
            assert!(false, "CFG_FILE is not set in .env file");
        }
    }
}

///
/// Package Definitions
///

#[derive(Clone, Deserialize, Debug, JsonSchema, PartialEq, PartialOrd)]
/// Definition of a Package
pub struct Package {
    /// Path of package directory
    pub cfg_path: String,
    /// Name of INI file
    pub ini_file: String,
    /// Name of Attribute Definition File
    pub json_file: String,
    /// Title of the package to be displayed on the webpage
    pub title: Option<String>,
    /// Name of the name of the service to be restarted by systemctl when the
    /// package is updated
    pub service_name: Option<String>,
}

/// Type alias based on a HashMap
pub type PackageHash = HashMap<String, Package>;

/// The structure that holds the definition of package items
#[allow(dead_code)]
pub struct Pkg {
    schema: Value,
    pub packages: Option<PackageHash>,
}

impl Pkg {
    /// Creates a new instance of the structure
    ///
    /// The type definition of PackageHash is used to create a compiled JSON schema that will be used
    /// to validate the Package definitions being loaded to PackageHash
    ///
    pub fn new<P: AsRef<Path> + std::fmt::Display>(def_path: P) -> Pkg {
        let schema = Self::create_packagehash_schema();
        let packages = Self::load_packages(def_path, &schema);
        Pkg { schema, packages }
    }

    /// Create a compiled JSON schema from Package definition
    /// via type alias PackageHash
    fn create_packagehash_schema() -> Value {
        let src_schema = schema_for!(PackageHash);
        create_json_schema(src_schema)
    }

    /// Load the package definitions from `def_path`
    fn load_packages<P: AsRef<Path> + std::fmt::Display>(
        def_path: P,
        schema: &Value,
    ) -> Option<PackageHash> {
        // Read JSON file
        let pkg = Self::read_defn_file(def_path, &schema);
        match pkg {
            Ok(packages) => {
                if packages.len() > 0 {
                    Some(Self::slugify_keys(packages))
                } else {
                    None
                }
            }
            Err(e) => {
                //log error text
                error!("{}", e);
                None
            }
        }
    }

    /// Read the contents of a file as JSON and, if valid against the schema, return an instance
    /// of 'PackageHash'
    fn read_defn_file<P: AsRef<Path> + std::fmt::Display>(
        path: P,
        schema: &Value,
    ) -> Result<PackageHash, CfgError> {
        // Open the file in read-only mode with buffer
        let f = File::open(path.as_ref());
        match f {
            Ok(file) => {
                let reader = BufReader::new(file);

                if let Ok(json_value) = serde_json::from_reader(reader) {
                    if jsonschema::is_valid(schema, &json_value) {
                        // Read the JSON contents of the file as an instance of 'PackageHash'.
                        if let Ok(pkg) = serde_json::from_value(json_value) {
                            Ok(pkg)
                        } else {
                            error!("conversion to struct failed for {}", path);
                            Err(CfgError::Schema(
                                "(failed to convert JSON to struct)".to_string(),
                            ))
                        }
                    } else {
                        let pathstr = path.as_ref().to_str().unwrap();
                        if let Ok(validator) = jsonschema::validator_for(schema) {
                            let result = validator.iter_errors(&json_value);
                            for error in result {
                                error!("{}", error);
                            }
                            error!("{} failed validation", pathstr);
                        }
                        Err(CfgError::Schema(pathstr.to_string()))
                    }
                } else {
                    error!("reading file {} as json failed", path);
                    Err(CfgError::Schema("(non-utf8 path)".to_string()))
                }
            }
            Err(e) => Err(CfgError::Io(e)),
        }
    }

    /// Slugify the package keys and update the 'title' field, if it is None,
    /// with the original key.
    fn slugify_keys(mut pkg: PackageHash) -> PackageHash {
        let mut pkg2: PackageHash = HashMap::new();
        for (k, v) in pkg.drain() {
            let sk = slugify(&k);
            let mut p = v.clone();
            if p.title.is_none() {
                p.title = Some(k);
            }
            pkg2.insert(sk, p);
        }
        pkg2
    }
}

#[cfg(test)]
mod test_pkg {
    use super::*;
    use env_logger::Target;
    use log::{info, LevelFilter};

    fn init_logging() {
        let _ = env_logger::builder()
            .target(Target::Stdout)
            .filter_level(LevelFilter::max())
            .is_test(true)
            .try_init();
    }

    #[test]
    #[ignore = "verbose output"]
    fn view_pkg_schema() {
        // Initialise Logger
        init_logging();

        let pkg_schema = schema_for!(PackageHash);
        info!("{}", serde_json::to_string_pretty(&pkg_schema).unwrap())
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
        // Initialise Logger
        init_logging();

        let schema = Pkg::create_packagehash_schema();
        let json_file = "tests/test2_pkg.json";
        let p = Pkg::read_defn_file(json_file, &schema).unwrap();
        assert_eq!(p.len(), 2);
        for (key, package) in p.iter() {
            info!(
                "Package: {} - {}, {}, {}, {}",
                key,
                package.cfg_path,
                package.ini_file,
                package.json_file,
                package.service_name.as_deref().unwrap_or("None")
            );
        }
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
