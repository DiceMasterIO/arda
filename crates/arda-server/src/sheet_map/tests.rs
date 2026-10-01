use super::*;
use serde_json::json;

const IDENTITY: &str = include_str!("../../mappings/identity.json");
const SRD: &str = include_str!("../../mappings/5e-srd-monster.json");

fn mapping(v: &Value) -> ServerResult<SheetMapping> {
    SheetMapping::from_json(&v.to_string())
}

fn refused(v: &Value) -> String {
    match mapping(v) {
        Err(ServerError::SheetMapping(e)) => e,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn base(rules: Value) -> Value {
    let mut v = json!({"format": "arda-sheet-mapping", "version": 1, "name": "t", "base": "copy",
           "tables": {"coin": {"gp": "gold", "sp": "silver", "cp": "copper"}}});
    v["rules"] = rules;
    v
}

#[test]
fn the_identity_mapping_returns_every_probe_unchanged() {
    let m = SheetMapping::from_json(IDENTITY).unwrap();
    assert_eq!(m.name(), "identity");
    for npc in probes().unwrap() {
        assert_eq!(m.apply(&npc).unwrap(), npc);
    }
}

#[test]
fn a_mapping_and_its_inverse_round_trip_every_probe() {
    let mut there = base(json!([
        {"to": "/sheet/ac", "from": "/sheet/armor_class"},
        {"to": "/stats/str", "from": "/sheet/abilities/0"},
        {"to": "/sheet/speed_m", "from": "/sheet/speed",
         "convert": [{"op": "scale", "factor": 0.3048, "round": 3}]},
        {"to": "/purse", "from": "/sheet/coins", "convert": [{"op": "keys", "table": "coin"}]}
    ]));
    there["drop"] = json!(["/sheet/armor_class", "/sheet/speed", "/sheet/coins"]);
    let mut back = base(json!([
        {"to": "/sheet/armor_class", "from": "/sheet/ac"},
        {"to": "/sheet/speed", "from": "/sheet/speed_m",
         "convert": [{"op": "scale", "factor": 3.280_839_895_013_123, "round": 0}]},
        {"to": "/sheet/coins", "from": "/purse", "convert": [{"op": "keys", "table": "rev"}]}
    ]));
    back["tables"]["rev"] = json!({"gold": "gp", "silver": "sp", "copper": "cp"});
    back["drop"] = json!(["/sheet/ac", "/sheet/speed_m", "/purse", "/stats"]);
    // `back` reads the mapped shape, so it skips the dry run over Arda NPCs.
    let back = SheetMapping::parse(&back.to_string()).unwrap();
    let there = mapping(&there).unwrap();
    for npc in probes().unwrap() {
        let mapped = there.apply(&npc).unwrap();
        assert!(mapped["sheet"].get("armor_class").is_none());
        assert_eq!(mapped["purse"]["gold"], npc["sheet"]["coins"]["gp"]);
        assert_eq!(mapped["stats"]["str"], npc["sheet"]["abilities"][0]);
        assert_eq!(back.apply(&mapped).unwrap(), npc);
    }
}

#[test]
fn the_srd_mapping_reshapes_every_probe() {
    let m = SheetMapping::from_json(SRD).unwrap();
    let mut stat_blocks = 0;
    for npc in probes().unwrap() {
        let out = m.apply(&npc).unwrap();
        let s = &npc["sheet"];
        assert_eq!(out["index"], npc["id"]);
        assert_eq!(out["strength"], s["abilities"][0]);
        assert_eq!(out["charisma"], s["abilities"][5]);
        assert_eq!(out["armor_class"][0]["value"], s["armor_class"]);
        assert_eq!(out["speed"]["walk"], format!("{} ft.", s["speed"]));
        assert_eq!(out["senses"]["passive_perception"], s["passive_perception"]);
        let profs = out["proficiencies"].as_array().unwrap();
        let skills = s["skills"].as_array().unwrap().len();
        let saves = s["saves"].as_array().unwrap();
        let proficient = saves.iter().filter(|x| x["proficient"] == true).count();
        assert_eq!(profs.len(), skills + proficient);
        for p in profs {
            let index = p["proficiency"]["index"].as_str().unwrap();
            assert!(index.starts_with("skill-") || index.starts_with("saving-throw-"));
            assert!(!index.contains(' '), "{index}");
        }
        let attacks = s["attacks"].as_array().unwrap();
        for (a, attack) in out["actions"].as_array().unwrap().iter().zip(attacks) {
            assert_eq!(
                a["damage"][0]["damage_type"]["index"],
                attack["damage_type"]
            );
            assert!(!a["damage"][0]["damage_dice"]
                .as_str()
                .unwrap()
                .contains(' '));
        }
        if s["kind"]["type"] == "stat_block" {
            stat_blocks += 1;
            assert!(
                out["challenge_rating"].is_number(),
                "{}",
                out["challenge_rating"]
            );
        } else {
            assert!(out.get("challenge_rating").is_none());
        }
        assert_eq!(
            out["spellcasting"].is_object(),
            s["spellcasting"].is_object()
        );
        assert_eq!(m.apply(&npc).unwrap(), out, "deterministic");
    }
    assert!(stat_blocks > 0, "the probes include stat-block commoners");
}

#[test]
fn bad_mappings_are_refused_with_the_rule_that_fails() {
    let mut wrong = base(json!([]));
    wrong["format"] = json!("nope");
    assert!(refused(&wrong).contains("format"));
    let mut version = base(json!([]));
    version["version"] = json!(2);
    assert!(refused(&version).contains("version 2"));
    let mut name = base(json!([]));
    name["name"] = json!("bad name\n");
    assert!(refused(&name).contains("name"));
    let mut typo = base(json!([]));
    typo["rulez"] = json!([]);
    assert!(refused(&typo).contains("unknown field"));
    for (rules, needle) in [
        (
            json!([{"to": "/a", "from": "/id", "const": 1}]),
            "rules[0]: give exactly one",
        ),
        (json!([{"to": "", "from": "/id"}]), "rules[0].to"),
        (json!([{"to": "/a", "from": "id"}]), "rules[0].from"),
        (
            json!([{"to": "/a", "from": "/id", "convert": [{"op": "table", "table": "nope"}]}]),
            "no table \"nope\"",
        ),
        (
            json!([{"to": "/a", "from": "/id", "convert": [{"op": "shout"}]}]),
            "unknown variant",
        ),
        (
            json!([{"to": "/a", "template": "{/name/given"}]),
            "unclosed",
        ),
        (
            json!([{"to": "/a", "from": "/sheet/nope"}]),
            "rules[0]: the source has nothing at \"/sheet/nope\"",
        ),
        (
            json!([{"to": "/a", "from": "/sheet/size", "convert": [{"op": "table", "table": "coin"}]}]),
            "is not in table",
        ),
        (
            json!([{"to": "/a", "const": 1, "where": {"/x": 1}}]),
            "`where` needs `fields`",
        ),
        (
            json!([{"to": "/a", "from": "/sheet/saves", "fields": [{"to": "/b", "from": "/nope"}]}]),
            "rules[0].fields[0]",
        ),
    ] {
        let e = refused(&base(rules.clone()));
        assert!(e.contains(needle), "{rules}: {e}");
    }
}

#[test]
fn optional_sources_skip_or_write_their_default() {
    let m = mapping(
        &json!({"format": "arda-sheet-mapping", "version": 1, "name": "o",
        "base": "empty", "rules": [
            {"to": "/cr", "from": "/sheet/kind/nothing", "optional": true},
            {"to": "/cr2", "from": "/sheet/kind/nothing", "optional": true, "default": "n/a"},
            {"to": "/k", "const": {"a": [1]}}
        ]}),
    )
    .unwrap();
    let out = m.apply(&probes().unwrap()[0]).unwrap();
    assert_eq!(out, json!({"cr2": "n/a", "k": {"a": [1]}}));
}

#[test]
fn bodies_are_unchanged_without_a_mapping_and_reshaped_with_one() {
    use http_body_util::BodyExt;
    let body = json!({"npcs": probes().unwrap()[..2].to_vec()});
    let plain = respond(None, &body, NpcAt::List("/npcs", None)).unwrap();
    assert!(plain.headers().get(X_ARDA_SHEET_MAPPING).is_none());
    let m = SheetMapping::from_json(SRD).unwrap();
    let mapped = respond(Some(&m), &body, NpcAt::List("/npcs", None)).unwrap();
    assert_eq!(mapped.headers()[X_ARDA_SHEET_MAPPING], "5e-srd-monster");
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let bytes =
        |r: Response| rt.block_on(async { r.into_body().collect().await.unwrap().to_bytes() });
    assert_eq!(bytes(plain), serde_json::to_vec(&body).unwrap());
    let mapped: Value = serde_json::from_slice(&bytes(mapped)).unwrap();
    assert_eq!(mapped["npcs"][1]["index"], body["npcs"][1]["id"]);
}
