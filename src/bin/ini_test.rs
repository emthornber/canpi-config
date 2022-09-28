//! # ini_test
//!
//! binary to investigate the rust-ini crate behaviour wrt sections
//!

use ini::Ini;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::fs;

const CFG_DATA: &str = r#"
canid=101
node_number=5432
start_event_id=2
node_mode=1
[network]
router_ssid = "home"
router_passwd = 123456
[apmode]
ap_ssid = canpi
ap_passwd = 654321
"#;

fn setup_file<P: AsRef<Path>>(test_file: P, data: &str) {
    let mut f = File::create(test_file).expect("file creation failed");
    f.write_all(data.as_bytes()).expect("file write failed");
}

fn teardown_file<P: AsRef<Path>>(test_file: P) {
    fs::remove_file(test_file).expect("file deletion failed");
}

/// Read the current canpi cfg values from file defined by 'path'
pub fn read_cfg_file<P: AsRef<Path>>(path: P) -> Result<Ini, ini::Error> {
    let cfg = Ini::load_from_file(path)?;
    Ok(cfg)
}

fn main() {
    let cfg_file = "scratch/ini_test.cfg";
    setup_file(&cfg_file, CFG_DATA);
    let ini_file = read_cfg_file(cfg_file);
    match ini_file {
        Ok(ini) => {
            let prop = ini.general_section();
            println!("\nGeneral Section");
            for (k, v) in prop.iter() {
                println!("{} = {}", k, v);
            }
            for (sec, prop) in ini.iter() {
                if let Some(s) = sec {
                println!("\nSection: {:?}", s);
                for (k, v) in prop.iter() {
                    println!("{} = {}", k, v);
                }
                }
            }
        }
        Err(e) => println!("Failed to read {}: {} ", cfg_file, e),
    }
    teardown_file(&cfg_file);
}
