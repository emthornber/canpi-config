use canpi_config::AttributeFormat::RegExp;
use canpi_config::*;
use dotenv::dotenv;
use std::fs::File;
use std::io::Write;
use std::path::Path;
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

fn setup_file<P: AsRef<Path>>(test_file: P, data: &str) {
    let mut f = File::create(test_file).expect("file creation failed");
    f.write_all(data.as_bytes()).expect("file write failed");
}

fn teardown_file<P: AsRef<Path>>(test_file: P) {
    fs::remove_file(test_file).expect("file deletion failed");
}

#[test]
/// Test filtering of attributes by action value via attributes_with_action()
fn attributes_with_action_test() {
    let cfg_file = "scratch/attributes_test.cfg";
    let defn_file = "scratch/attributes_test.json";
    setup_file(&defn_file, DEFN_DATA);
    setup_file(&cfg_file, CFG_DATA);
    let cfg = Cfg::new(&cfg_file, &defn_file);
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
fn write_attr_good() {
    let cfg_file = "scratch/wattr_test.cfg";
    let defn_file = "scratch/wattr_test.json";
    setup_file(&defn_file, DEFN_DATA);
    setup_file(&cfg_file, CFG_DATA);
    let mut cfg = Cfg::new(&cfg_file, &defn_file);
    let start_event_id = cfg.read_attribute("start_event_id".to_string());
    if let Some(sei) = start_event_id {
        assert_eq!(sei.prompt, "Start Event Id", "Field 'prompt'");
        assert_eq!(sei.current, "2", "Field 'current'");
        assert_eq!(sei.default, "1", "Field 'default'");
    }
    let new_start_event_id = canpi_config::Attribute {
        prompt: "sTART eVENT iD".to_string(),
        tooltip: "new tooltip".to_string(),
        current: "1".to_string(),
        default: "2".to_string(),
        format: RegExp("[1-8]".to_string()),
        action: ActionBehaviour::Hide,
    };
    cfg.write_attribute("start_event_id".to_string(), &new_start_event_id)
        .expect("attribute write failed");
    let new_start_event_id = cfg.read_attribute("start_event_id".to_string());
    if let Some(nsei) = new_start_event_id {
        assert_eq!(nsei.prompt, "sTART eVENT iD", "Field 'prompt'");
        assert_eq!(nsei.current, "1", "Field 'current'");
        assert_eq!(nsei.default, "2", "Field 'default'");
    }
    teardown_file(&cfg_file);
    teardown_file(&defn_file);
}

#[test]
#[should_panic]
#[ignore = "Invalid test"]
fn write_attr_bad() {
    let cfg_file = "scratch/wattr_test.cfg";
    let defn_file = "scratch/wattr_test.json";
    let mut cfg = Cfg::new(&cfg_file, &defn_file);
    let new_start_event_id = canpi_config::Attribute {
        prompt: "sTART eVENT iD".to_string(),
        tooltip: "new tooltip".to_string(),
        current: "1".to_string(),
        default: "2".to_string(),
        format: RegExp("[1-8]".to_string()),
        action: ActionBehaviour::Hide,
    };
    cfg.write_attribute("start_event_id".to_string(), &new_start_event_id)
        .expect("attribute write failed");
}

#[test]
fn write_ini_file() {
    dotenv().ok();
    let mut cfg_file = env::var("CFG_FILE").expect("CFG_FILE is not set in .env file");
    let def_file = env::var("DEF_FILE").expect("DEF_FILE is not set in .env file");
    let cfg = Cfg::new(cfg_file.clone(), def_file);
    cfg_file.push_str(".new");
    cfg.write_cfg_file(cfg_file, Some(true))
        .expect("Failed to write cfg file");
}

/// Load `cfg_data` with DEFN_DATA from files named after `name`, returning the Cfg and the paths
fn load_test_files(name: &str, cfg_data: &str) -> (Cfg, String, String) {
    let cfg_file = format!("scratch/{}.cfg", name);
    let defn_file = format!("scratch/{}.json", name);
    setup_file(&defn_file, DEFN_DATA);
    setup_file(&cfg_file, cfg_data);
    let cfg = Cfg::new(cfg_file.as_str(), defn_file.as_str());
    (cfg, cfg_file, defn_file)
}

/// Set the current value of `key`, as the web app does
fn set_current(cfg: &mut Cfg, key: &str, value: &str) {
    let mut a = cfg
        .read_attribute(key.to_string())
        .expect("attribute missing")
        .clone();
    a.current = value.to_string();
    cfg.write_attribute(key.to_string(), &a)
        .expect("attribute write failed");
}

fn saved_value(cfg_file: &str, key: &str) -> Option<String> {
    Cfg::read_cfg_file(cfg_file)
        .expect("saved file not readable")
        .get(key)
        .cloned()
}

#[test]
/// A key the definition does not cover is kept when the file is saved
fn save_keeps_keys_not_defined() {
    let (mut cfg, cfg_file, defn_file) = load_test_files(
        "save_keep_test",
        "canid=101\nnode_number=5432\nstart_event_id=2\nnode_mode=1\nlog_console=true\n",
    );
    set_current(&mut cfg, "start_event_id", "3");
    cfg.write_cfg_file(&cfg_file, Some(false))
        .expect("save failed");
    assert_eq!(
        saved_value(&cfg_file, "log_console").as_deref(),
        Some("true")
    );
    assert_eq!(
        saved_value(&cfg_file, "start_event_id").as_deref(),
        Some("3")
    );
    assert_eq!(saved_value(&cfg_file, "canid").as_deref(), Some("101"));
    teardown_file(&cfg_file);
    teardown_file(&defn_file);
}

#[test]
/// An item missing from the file is shown with the definition's value and written when saved
fn save_writes_items_missing_from_file() {
    let (cfg, cfg_file, defn_file) = load_test_files(
        "save_missing_test",
        "canid=101\nnode_number=5432\nstart_event_id=2\n",
    );
    let node_mode = cfg
        .read_attribute("node_mode".to_string())
        .expect("node_mode not loaded");
    assert_eq!(node_mode.current, "0");
    cfg.write_cfg_file(&cfg_file, Some(false))
        .expect("save failed");
    assert_eq!(saved_value(&cfg_file, "node_mode").as_deref(), Some("0"));
    assert_eq!(saved_value(&cfg_file, "canid").as_deref(), Some("101"));
    teardown_file(&cfg_file);
    teardown_file(&defn_file);
}

#[test]
/// A file that could not be read is not overwritten with the definition's values
fn save_refuses_unreadable_file() {
    let data = "canid=101\n[broken\nnode_number=5432\n";
    let (mut cfg, cfg_file, defn_file) = load_test_files("save_unreadable_test", data);
    set_current(&mut cfg, "start_event_id", "3");
    assert!(cfg.write_cfg_file(&cfg_file, Some(true)).is_err());
    assert_eq!(fs::read_to_string(&cfg_file).expect("file gone"), data);
    assert!(!Path::new(&format!("{}.tmp", cfg_file)).exists());
    assert!(!Path::new(&format!("{}.bak", cfg_file)).exists());
    teardown_file(&cfg_file);
    teardown_file(&defn_file);
}

#[test]
/// A value changed in the file by another program after loading is kept, unless edited here
fn save_keeps_changes_made_by_another_program() {
    let (mut cfg, cfg_file, defn_file) = load_test_files("save_other_test", CFG_DATA);
    set_current(&mut cfg, "start_event_id", "5");
    // canpid changes the CAN Id after the web app has loaded the file
    setup_file(
        &cfg_file,
        "canid=150\nnode_number=5432\nstart_event_id=2\nnode_mode=1\n",
    );
    cfg.write_cfg_file(&cfg_file, Some(false))
        .expect("save failed");
    assert_eq!(saved_value(&cfg_file, "canid").as_deref(), Some("150"));
    assert_eq!(
        saved_value(&cfg_file, "start_event_id").as_deref(),
        Some("5")
    );
    teardown_file(&cfg_file);
    teardown_file(&defn_file);
}

#[test]
/// The file is replaced through a temporary file, and the backup is a copy of the old file
fn save_replaces_file_and_keeps_backup() {
    let old = "canid=101\nnode_number=5432\nstart_event_id=2\nnode_mode=1\n";
    let (mut cfg, cfg_file, defn_file) = load_test_files("save_backup_test", old);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&cfg_file, fs::Permissions::from_mode(0o640)).expect("chmod failed");
    }
    set_current(&mut cfg, "start_event_id", "7");
    cfg.write_cfg_file(&cfg_file, Some(true))
        .expect("save failed");
    let backup = format!("{}.bak", cfg_file);
    assert_eq!(fs::read_to_string(&backup).expect("no backup"), old);
    assert!(!Path::new(&format!("{}.tmp", cfg_file)).exists());
    assert_eq!(
        saved_value(&cfg_file, "start_event_id").as_deref(),
        Some("7")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&cfg_file)
            .expect("no file")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o640);
    }
    teardown_file(&backup);
    teardown_file(&cfg_file);
    teardown_file(&defn_file);
}
