use canpi_config;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use canpi_config::Cfg;

fn read_value_from_file<P: AsRef<Path>>(path: P) -> Result<Value, canpi_config::CanPiCfgError> {
    // Open the file in read-only mode with buffer
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    // Read the JSON contents of the file
    let v = serde_json::from_reader(reader)?;

    // Return the 'Value'
    Ok(v)
}

#[test]
fn validate_good_cfg() {
    let cfg_json: Value = read_value_from_file("tests/good-example-config-defn.json".to_string()).unwrap();
    let cfg = Cfg::new();

    assert!(cfg.validate_defn_file(cfg_json));
}

#[test]
fn validate_bad_cfg() {
    let cfg_json: Value = read_value_from_file("tests/bad-example-config-defn.json".to_string()).unwrap();
    let cfg = Cfg::new();

    assert!(!cfg.validate_defn_file(cfg_json));
}

