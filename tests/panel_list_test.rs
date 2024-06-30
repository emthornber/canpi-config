use canpi_config::PanelList;

#[test]
fn panel_list_new() {
    let panel_dir = "tests/";
    let panel_list = PanelList::new(panel_dir);
    match panel_list.panels {
        Some(p) => {
            assert_eq!(p.len(), 1);
            assert!(p.contains_key(&1));
        }
        None => assert!(false),
    }
}
