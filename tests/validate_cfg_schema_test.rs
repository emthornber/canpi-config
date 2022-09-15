use canpi_config;

#[test]
fn validate_good_cfg() {
    let cfg_file = "static/canpi-config-defn.json".to_string();
    let schema_file: String = "static/canpi-config-schema.json".to_string();

    assert!(canpi_config::validate_defn_file(schema_file, cfg_file);
}

#[test]
fn validate_bad_cfg() {
        let cfg_file = "tests/canpi-config-bad-defn.json".to_string();
    let schema_file: String = "static/canpi-config-schema.json".to_string();

    //    println!("Loaded config file '{}'", cfg_file);
    let _status = match canpi_config::validate_defn_file(schema_file, cfg_file) {
        Ok(()) => { assert!(false) }
        Err(_e) => { assert!(true) }
    };

}

