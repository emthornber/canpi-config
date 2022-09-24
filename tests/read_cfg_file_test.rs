use canpi_config;
use dotenv::dotenv;
use std::env;

#[test]
fn it_reads_cfg() {
    dotenv().ok();
    let cfg_file = env::var("CFG_FILE").expect("CFG_FILE is not set in .env file");
    //    println!("Loaded config file '{}'", cfg_file);
    let cfg = canpi_config::Cfg::read_cfg_file(cfg_file).expect("Deserialize failed");
    /*
       for (sec, prop) in cfg.iter() {
           println!("Section '{:?}'", sec);
           for (k, v) in prop.iter() {
               println!("{}={}", k, v);
           }
       }
    */
    assert_eq!(
        cfg.get_from(None::<String>, "router_ssid"),
        Some("home")
    );
}
