use super::*;

#[test]
fn committed_schemas_are_fresh() {
    let dir = schema_dir();
    let files = render().unwrap();
    if std::env::var_os("ARDA_BLESS_BINDINGS").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        for (name, text) in &files {
            std::fs::write(dir.join(name), text).unwrap();
        }
    }
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}; bless with ARDA_BLESS_BINDINGS=1", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut expected: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
    expected.sort();
    assert_eq!(
        on_disk, expected,
        "stale file set; rerun with ARDA_BLESS_BINDINGS=1"
    );
    for (name, text) in &files {
        let committed = std::fs::read_to_string(dir.join(name)).unwrap();
        assert_eq!(
            &committed, text,
            "{name} is stale; rerun with ARDA_BLESS_BINDINGS=1"
        );
    }
}

#[test]
fn rendering_is_deterministic_and_every_schema_is_2020_12() {
    assert_eq!(render().unwrap(), render().unwrap());
    for e in catalog().unwrap() {
        assert_eq!(e.schema["$schema"], DIALECT, "{}", e.name);
        assert_eq!(e.schema["title"], e.name);
        assert!(!e.routes.is_empty(), "{}", e.name);
    }
}

#[test]
fn every_schema_name_is_a_typescript_binding() {
    let ts: Vec<String> = crate::bindings::render()
        .unwrap()
        .into_iter()
        .map(|(n, _)| n.trim_end_matches(".ts").to_owned())
        .collect();
    let domain = [
        "Settlement",
        "SettlementSociety",
        "TownPlan",
        "SettlementList",
        "SettlementDetail",
        "SettlementNpcs",
    ];
    for e in catalog().unwrap() {
        assert!(
            ts.iter().any(|t| t == e.name) || domain.contains(&e.name),
            "{} has no TS binding",
            e.name
        );
    }
}

#[test]
fn scene_runs_are_count_value_pairs_and_seeds_are_strings() {
    let scene = catalog()
        .unwrap()
        .into_iter()
        .find(|e| e.name == "SceneDto")
        .unwrap()
        .schema;
    let text = scene.to_string();
    assert!(text.contains("prefixItems"), "{text}");
    assert_eq!(scene["properties"]["seed"]["type"], "string");
    let index = index(&catalog().unwrap());
    assert_eq!(index["format_version"], INDEX_FORMAT);
    assert!(index["schemas"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["url"] == "/v1/schema/TacticalScene.json"));
}
