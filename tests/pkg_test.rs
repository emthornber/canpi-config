use canpi_config::Pkg;

#[test]
fn pkg_new() {
    let def_path = "tests/test_pkg.json";
    let pkg = Pkg::new(def_path);
    match pkg.packages {
        Some(p) => {
            assert_eq!(p.len(), 2);
            assert!(p.contains_key("AutoHotSpot"));
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
            assert!(p.contains_key("CANPiServer"));
            let canpi_pkg = p.get("CANPiServer").unwrap();
            if let Some(sn) = &canpi_pkg.service_name {
                assert_eq!(sn, "canpid.service");
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
