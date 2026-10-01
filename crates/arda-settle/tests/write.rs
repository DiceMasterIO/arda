//! Review round 2 #31: `write` encodes every file before it writes the
//! first, so the directory never holds a mix of new and old files after an
//! encoding failure; what it writes is exactly what `encode` produced.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_settle::{encode, run, synthetic, write, PlaceParams};

#[test]
fn write_stores_exactly_the_encoded_files() {
    let params = PlaceParams {
        seed: 7,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let society = run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!("arda-settle-write-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let files = encode(&dir, &grid, params.seed, &society).unwrap();
    assert!(!dir.exists(), "encoding touches no file");
    write(&dir, &grid, params.seed, &society).unwrap();
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    let mut names: Vec<String> = files.iter().map(|(n, _)| (*n).to_owned()).collect();
    names.sort();
    assert_eq!(on_disk, names);
    for (name, bytes) in &files {
        assert_eq!(&std::fs::read(dir.join(name)).unwrap(), bytes, "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
