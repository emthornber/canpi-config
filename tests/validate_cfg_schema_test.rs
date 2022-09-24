use canpi_config;
use jsonschema::{Draft, JSONSchema, output::BasicOutput};
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

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
    let schema_json: Value = read_value_from_file("static/config-defn-schema.json".to_string()).unwrap();

    assert!(canpi_config::Cfg::validate_defn_file(schema_json, cfg_json));
}

#[test]
fn display_good_cfg() {
    let cfg_json: Value = read_value_from_file("tests/bad-example-config-defn.json".to_string()).unwrap();
    let schema_json: Value = read_value_from_file("static/config-defn-schema.json".to_string()).unwrap();
    let compiled_schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema_json)
        .expect("A valid schema");
    let output: BasicOutput = compiled_schema.apply(&cfg_json).basic();
    match output {
        BasicOutput::Valid(annotations) => {
            for annotation in annotations {
                println!(
                    "Value: {} at path {}",
                    annotation.value(),
                    annotation.instance_location()
                )
            }
        },
        BasicOutput::Invalid(errors) => {
            for error in errors {
                println!(
                    "Error: {} at path {}",
                    error.error_description(),
                    error.instance_location()
                )
            }
        }
    }
}

#[test]
fn validate_bad_cfg() {
    let cfg_json: Value = read_value_from_file("tests/bad-example-config-defn.json".to_string()).unwrap();
    let schema_json: Value = read_value_from_file("static/config-defn-schema.json".to_string()).unwrap();

    //    println!("Loaded config file '{}'", cfg_file);
    let _status = match canpi_config::Cfg::validate_defn_file(schema_json, cfg_json) {
        true => { assert!(false) }
        false => { assert!(true) }
    };
}

