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
        None => assert!(false),
    }
}
