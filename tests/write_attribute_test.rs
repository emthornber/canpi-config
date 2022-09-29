use canpi_config::*;
use std::io::Write;
use std::fs;
use std::fs::File;
use std::path::Path;
use canpi_config::ActionBehaviour;

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
fn write_attr_good() {
    let cfg_file = "scratch/wattr_test.cfg";
    let defn_file = "scratch/wattr_test.json";
    setup_file(&defn_file, DEFN_DATA);
    setup_file(&cfg_file, CFG_DATA);
    let mut cfg = Cfg::new();
    cfg.load_configuration(&cfg_file, &defn_file)
        .expect("parameter definition failed to load");
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
        format: "[1-8]".to_string(),
        action: ActionBehaviour::Hide,
    };
    cfg.write_attribute("start_event_id".to_string(), &new_start_event_id).expect("attribute write failed");
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
fn write_attr_bad() {
    let mut cfg = Cfg::new();
    let new_start_event_id = canpi_config::Attribute {
        prompt: "sTART eVENT iD".to_string(),
        tooltip: "new tooltip".to_string(),
        current: "1".to_string(),
        default: "2".to_string(),
        format: "[1-8]".to_string(),
        action: ActionBehaviour::Hide,
    };
    cfg.write_attribute("start_event_id".to_string(), &new_start_event_id).expect("attribute write failed");
}