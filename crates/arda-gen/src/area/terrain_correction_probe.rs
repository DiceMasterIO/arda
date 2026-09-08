//! Explicit local measurement; final water acceptance uses the complete world.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::continent::{
    build_continent,
    bundles::{coarse_height, refine_height},
};
use arda_core::{formats::cells::CELL_BYTES, GenerateConfig};
use std::{
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
};

#[test]
#[ignore = "bounded real terrain correction measurement; run manually in release"]
fn reported_boundary_patch() {
    const N: usize = 1536;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = repo.join("docs/capstone/features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/boundary-patch");
    fs::create_dir(&report).expect("new evidence directory; never overwrite prior probe");
    let start = std::time::Instant::now();
    let continent = build_continent(436342, GenerateConfig::default(), 0);
    let mut initial = Vec::with_capacity(N * N);
    let mut coarse = Vec::with_capacity(N * N);
    for y in 0..N {
        for x in 0..N {
            let ax = 2 * 512 + i32::try_from(x).unwrap();
            let ay = 9 * 512 + i32::try_from(y).unwrap();
            let h = coarse_height(&continent.grid, ax, ay);
            initial.push(refine_height(436342, h, ax, ay));
            coarse.push(h);
        }
    }
    let mut evolved = initial.clone();
    let before = std::time::Instant::now();
    super::evolution::evolve(&mut evolved, &coarse, N, N).unwrap();
    let evolution_seconds = before.elapsed().as_secs_f64();
    let mut previous = vec![0; N * N];
    for ty in 0..3 {
        for tx in 0..3 {
            let path = repo.join(format!("out/area-water-terrain-realism/candidate-static-vtt-02/seed-436342/world/areas/{:02}_{:02}/cells.bin", 2 + tx, 9 + ty));
            let bytes = fs::read(path).unwrap();
            assert_eq!(bytes.len(), 512 * 512 * CELL_BYTES);
            for y in 0..512 {
                for x in 0..512 {
                    let offset = (y * 512 + x) * CELL_BYTES;
                    previous[(ty * 512 + y) * N + tx * 512 + x] =
                        i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
                }
            }
        }
    }
    for (name, heights) in [
        ("initial", &initial),
        ("shared", &evolved),
        ("candidate02", &previous),
    ] {
        let mut file =
            BufWriter::new(fs::File::create(report.join(format!("{name}.i32le"))).unwrap());
        for h in heights {
            file.write_all(&h.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();
    }
    let mut profile = Vec::new();
    for x in 448..=576 {
        let mut delta = Vec::new();
        let mut old = Vec::new();
        for y in 576..960 {
            let i = y * N + x;
            delta.push(i64::from(evolved[i]) - i64::from(initial[i]));
            old.push(i64::from(previous[i]) - i64::from(initial[i]));
        }
        delta.sort_unstable();
        old.sort_unstable();
        profile.push(serde_json::json!({"relative_x":x, "new_median_delta_mm":delta[delta.len()/2], "previous_median_delta_mm":old[old.len()/2]}));
    }
    let report_json = serde_json::json!({
        "seed":436342,"attempt":0,"origin_cells":[1024,4608],"width":N,"height":N,
        "target_area":[3,10],"evolution_seconds":evolution_seconds,"total_seconds":start.elapsed().as_secs_f64(),
        "scope":"1536-square physical terrain probe; cropped catchment and normalization, no final annual water solve. Candidate02 differs in finest octave and evolution partition.",
        "profile":profile
    });
    fs::write(
        report.join("measurements.json"),
        serde_json::to_vec_pretty(&report_json).unwrap(),
    )
    .unwrap();
    println!("{}", report_json);
}
