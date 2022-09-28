use canpi_config;
use canpi_config::Cfg;
use dotenv::dotenv;
use std::env;

#[test]
fn load_configuration_test() {
    dotenv().ok();
    let cfg_file = env::var("CFG_FILE").expect("CFG_FILE is not set in .env file");
    let def_file = env::var("DEF_FILE").expect("DEF_FILE is not set in .env file");

    let mut cfg = Cfg::new();
    cfg.load_configuration(cfg_file, def_file)
        .expect("Loading configuration");

    let attr = cfg.get_attribute("router_ssid".to_string());
    if let Some(a) = attr {
        assert_eq!(a.current, "home");
    } else {
        assert!(false);
    }
}
