use canpi_config;
use canpi_config::Cfg;
use dotenv::dotenv;
use std::env;

#[test]
fn it_reads_cfg() {
    dotenv().ok();
    let cfg_file = env::var("CFG_FILE").expect("CFG_FILE is not set in .env file");
    let def_file = env::var("DEF_FILE").expect("DEF_FILE is not set in .env file");

    let mut cfg = Cfg::new();
    cfg.load_configuration(cfg_file, def_file)
        .expect("Loading configuration");

    //    println!("Loaded config file '{}'", cfg_file);
    /*
       for (sec, prop) in cfg.iter() {
           println!("Section '{:?}'", sec);
           for (k, v) in prop.iter() {
               println!("{}={}", k, v);
           }
       }
    */
    let attr = cfg.get_attribute("router_ssid".to_string());
    if let Some(a) = attr {
        assert_eq!(a.current, "home");
    } else {
        assert!(false);
    }
}
