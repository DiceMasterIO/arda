use super::*;
use serde_json::json;

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
        "Realm",
        "RealmList",
        "RealmDetail",
        "RealmSociety",
        "Relation",
        "Event",
        "Gazetteer",
        "GazetteerFeature",
        "GazetteerPoint",
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

#[test]
fn the_index_reads_as_schema_index_and_lists_itself() {
    let entries = catalog().unwrap();
    let body: SchemaIndex = serde_json::from_value(index(&entries)).unwrap();
    assert_eq!(body.format_version, INDEX_FORMAT);
    assert_eq!(body.schemas.len(), entries.len());
    let route = |name: &str| {
        body.schemas
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} is not listed"))
            .routes
            .clone()
    };
    assert_eq!(route("SchemaIndex"), ["GET /v1/schema"]);
    assert_eq!(route("DungeonDto"), ["GET /v1/tactical/dungeon"]);
    assert!(route("SceneDto").contains(&"GET /v1/tactical/dungeon/scene".to_owned()));
}

#[test]
fn the_dungeon_seed_is_a_string() {
    let dungeon = catalog()
        .unwrap()
        .into_iter()
        .find(|e| e.name == "DungeonDto")
        .unwrap()
        .schema;
    let params = &dungeon["$defs"]["DungeonParamsDto"];
    assert_eq!(params["properties"]["seed"]["type"], "string", "{dungeon}");
}

/// The committed baseline of the frozen `/v1` schemas.
fn baseline_dir() -> PathBuf {
    schema_dir().join("../schema-baseline/v1")
}

/// The current schemas as a [`compat::Snapshot`].
fn current_snapshot() -> compat::Snapshot {
    let entries = catalog().unwrap();
    compat::Snapshot {
        schemas: entries
            .iter()
            .map(|e| (e.name.to_owned(), e.schema.clone()))
            .collect(),
        families: entries
            .iter()
            .map(|e| (e.name.to_owned(), e.family.to_owned()))
            .collect(),
        versions: crate::versions::family_versions()
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    }
}

/// `versions.json` of the baseline: `{families, versions}`.
fn manifest(s: &compat::Snapshot) -> Value {
    json!({ "families": s.families, "versions": s.versions })
}

#[test]
fn frozen_schemas_keep_their_shape_unless_their_family_is_bumped() {
    let dir = baseline_dir();
    let now = current_snapshot();
    if std::env::var_os("ARDA_BLESS_SCHEMA_BASELINE").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        for (name, schema) in &now.schemas {
            std::fs::write(dir.join(format!("{name}.json")), pretty(schema).unwrap()).unwrap();
        }
        std::fs::write(dir.join("versions.json"), pretty(&manifest(&now)).unwrap()).unwrap();
    }
    let read = |path: PathBuf| -> Value {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e}; create the baseline with ARDA_BLESS_SCHEMA_BASELINE=1",
                path.display()
            )
        });
        serde_json::from_str(&text).unwrap()
    };
    let m = read(dir.join("versions.json"));
    let strings = |v: &Value| -> std::collections::BTreeMap<String, String> {
        serde_json::from_value(v.clone()).unwrap()
    };
    let mut old = compat::Snapshot {
        families: strings(&m["families"]),
        versions: strings(&m["versions"]),
        ..compat::Snapshot::default()
    };
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if name != "versions" {
            old.schemas.insert(name, read(path));
        }
    }
    assert_eq!(
        old.schemas.len(),
        old.families.len(),
        "baseline files and versions.json disagree"
    );
    let breaks = compat::unexcused(&old, &now, &crate::versions::is_open);
    assert!(
        breaks.is_empty(),
        "breaking /v1 schema changes without a family version bump \
         (docs/capstone/versioning.md):\n{}\nAdd the field instead, or bump the family's \
         version constant, then re-baseline with ARDA_BLESS_SCHEMA_BASELINE=1",
        breaks.join("\n")
    );
}

#[test]
fn every_schema_names_a_known_family() {
    let families: Vec<&str> = crate::versions::family_versions()
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    for e in catalog().unwrap() {
        assert!(families.contains(&e.family), "{}: {}", e.name, e.family);
    }
}

#[test]
fn every_open_enum_exists_and_the_policy_page_lists_it() {
    let policy = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/capstone/versioning.md"),
    )
    .unwrap();
    let all = catalog().unwrap();
    for (name, fallback) in crate::versions::OPEN_ENUMS {
        assert!(
            all.iter().any(|e| e.schema["$defs"].get(*name).is_some()),
            "{name} is not a schema $defs entry"
        );
        assert!(
            policy.contains(&format!("`{name}` | `{fallback}`")),
            "versioning.md does not list open enum {name} with fallback {fallback}"
        );
    }
}
