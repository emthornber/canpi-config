use canpi_config::Pkg;

#[test]
fn pkg_new1() {
    let def_path = "tests/test_pkg.json";
    let pkg = Pkg::new(def_path);
    match pkg.packages {
        Some(p) => {
            assert_eq!(p.len(), 2);
            assert!(p.contains_key("auto-wap"));
        }
        None => assert!(false, "Packages should not be None"),
    }
}

#[test]
fn pkg_new1_list() {
    let def_path = "tests/test_pkg.json";
    let pkg = Pkg::new(def_path);
    match pkg.packages {
        Some(p) => {
            assert_eq!(p.len(), 2);
            let keys: Vec<&String> = p.keys().collect();
            assert!(keys.contains(&&"auto-wap".to_string()));
            let autowap_pkg = p.get("auto-wap").unwrap();
            if let Some(t) = &autowap_pkg.title {
                assert_eq!(t, "Hotspot");
            } else {
                assert!(false, "Title should not be None");
            }
        }
        None => assert!(false, "Packages should not be None"),
    }
}

#[test]
fn pkg_new2() {
    let def_path = "tests/test2_pkg.json";
    let pkg = Pkg::new(def_path);
    match pkg.packages {
        Some(p) => {
            assert_eq!(p.len(), 2);
            assert!(p.contains_key("canpi-server"));
            let canpi_pkg = p.get("canpi-server").unwrap();
            if let Some(t) = &canpi_pkg.title {
                assert_eq!(t, "CANPi Server");
            } else {
                assert!(false, "Title should not be None");
            }
            if let Some(sn) = &canpi_pkg.service_name {
                assert_eq!(sn, "canpid");
            } else {
                assert!(false, "Service name should not be None");
            }
        }
        None => assert!(false, "Packages should not be None"),
    }
}

#[test]
fn pkg_new3() {
    let def_path = "tests/test2_pkg.json";
    let pkg = Pkg::new(def_path);
    match pkg.packages {
        Some(p) => {
            assert_eq!(p.len(), 2);
            assert!(p.contains_key("AutoHotSpot"));
            let canpi_pkg = p.get("AutoHotSpot").unwrap();
            if let Some(_sn) = &canpi_pkg.service_name {
                assert!(false, "Service name should be None for AutoHotSpot");
            } else {
                assert!(true, "Service name not defined for AutoHotSpot");
            }
        }
        None => assert!(false, "Packages should not be None"),
    }
}
