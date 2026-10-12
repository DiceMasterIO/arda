//! Adding the gazetteer must not change one existing name: the bytes of
//! `names.json` for the synthetic world are pinned (logic/15, names are
//! drawn in a fixed order and the gazetteer's come last).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::output::{json_bytes, NamesFile, FORMAT_VERSION};
use arda_settle::{run, synthetic, PlaceParams};
use std::path::Path;

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

const NAMES_JSON: u64 = 0x6e87212e1145193f;
const OTHER_NAMES: u64 = 5_113_561_676_284_192_723;

#[test]
fn names_json_bytes_are_pinned() {
    let params = PlaceParams {
        seed: 7,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let s = run(&grid, params).unwrap();
    let file = NamesFile {
        format_version: FORMAT_VERSION,
        rivers: s.rivers.clone(),
        mountains: s.mountains.clone(),
        regions: s.regions.clone(),
    };
    let bytes = json_bytes(Path::new("names.json"), &file).unwrap();
    let mut others: Vec<(&str, &str)> = Vec::new();
    others.extend(
        s.settlements
            .iter()
            .map(|x| (x.name.as_str(), x.name_gloss.as_str())),
    );
    others.extend(
        s.passes
            .iter()
            .map(|x| (x.name.as_str(), x.name_gloss.as_str())),
    );
    others.extend(
        s.realms
            .realms
            .iter()
            .map(|x| (x.name.as_str(), x.name_gloss.as_str())),
    );
    let settle = serde_json::to_vec(&others).unwrap();
    assert_eq!(fnv(&bytes), NAMES_JSON);
    assert_eq!(fnv(&settle), OTHER_NAMES);
}
