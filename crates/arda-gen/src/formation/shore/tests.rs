use super::*;

fn setting(exposure: u8, sediment: u8, rock: u8) -> Setting {
    Setting {
        exposure,
        sediment,
        hinterland_m: 0,
        nearshore_depth_mm: 5_000,
        rock,
        dist_m: 0,
    }
}

fn cell(land: bool, rise_m: i32, sea_share: i32, s: Setting) -> ShoreCell {
    ShoreCell {
        land,
        z_mm: if land { 2_000 } else { -1_000 },
        rise_m,
        sea_share,
        mouth_km2: 0,
        sand_body: false,
        delta: false,
        setting: s,
    }
}

#[test]
fn high_ground_meeting_exposed_sea_is_cliff() {
    assert_eq!(
        classify(&cell(true, 40, 500, setting(150, 30, 150))),
        ShoreClass::Cliff
    );
}

#[test]
fn low_embayed_shores_are_beaches_and_exposed_hard_rock_gives_shingle() {
    assert_eq!(
        classify(&cell(true, 3, 350, setting(90, 20, 100))),
        ShoreClass::SandBeach,
        "a bay without a river still collects sand"
    );
    assert_eq!(
        classify(&cell(true, 6, 500, setting(200, 80, 160))),
        ShoreClass::ShingleBeach
    );
    assert_eq!(
        classify(&cell(true, 12, 600, setting(120, 20, 150))),
        ShoreClass::RockyShore,
        "a moderate headland without sediment is rocky"
    );
}

#[test]
fn sheltered_muddy_margins_are_marsh_and_flats_and_mouths_estuaries() {
    let quiet = setting(30, 120, 120);
    assert_eq!(classify(&cell(true, 2, 400, quiet)), ShoreClass::Marsh);
    assert_eq!(classify(&cell(false, 0, 400, quiet)), ShoreClass::TidalFlat);
    let mut mouth = cell(false, 0, 400, setting(120, 200, 120));
    mouth.mouth_km2 = 300;
    assert_eq!(classify(&mouth), ShoreClass::Estuary);
}

#[test]
fn survey_finds_a_cliffed_coast_and_a_barrier_island() {
    // Mainland plateau (40 m) in the north; a barrier strip offshore.
    let (w, h) = (512, 512);
    let mut g = Lattice::new(w, h, 39_062_500).unwrap();
    let mut barrier = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            g.z[i] = if y < 200 {
                40_000
            } else if (260..266).contains(&y) && (100..400).contains(&x) {
                barrier.push(i as u32);
                2_000
            } else {
                -8_000
            };
        }
    }
    let built = Builders {
        barrier_cells: &barrier,
        ..Builders::default()
    };
    let layer = survey(&g, 100_000_000, 3, &built).unwrap();
    assert_eq!(layer.islands.len(), 1);
    assert_eq!(layer.islands[0].cause, IslandCause::Barrier);
    let cliffs = layer
        .classes
        .iter()
        .filter(|&&c| c == ShoreClass::Cliff)
        .count();
    assert!(cliffs > 50, "the plateau edge is cliffed: {cliffs}");
}

#[test]
fn low_headlands_keep_beaches_only_on_sand_bodies_or_at_mouths() {
    // Regression (seed-42 full size: 148‰ beach in bays, 200‰ on
    // headlands): any sediment within 6 km made a low headland a beach.
    let fed = setting(120, 200, 120);
    let point = cell(true, 3, 700, fed);
    assert_eq!(classify(&point), ShoreClass::RockyShore);
    let mouth = ShoreCell {
        mouth_km2: 40,
        ..point
    };
    assert_eq!(classify(&mouth), ShoreClass::SandBeach);
    let barrier = ShoreCell {
        sand_body: true,
        ..point
    };
    assert_eq!(classify(&barrier), ShoreClass::SandBeach);
    // A straight low shore with sediment keeps its beach.
    assert_eq!(classify(&cell(true, 3, 500, fed)), ShoreClass::SandBeach);
    // Sheltered delta shores are marsh; the exposed front is sand.
    let delta = ShoreCell {
        delta: true,
        sand_body: true,
        ..cell(true, 2, 400, setting(60, 200, 120))
    };
    assert_eq!(classify(&delta), ShoreClass::Marsh);
    let front = ShoreCell {
        delta: true,
        sand_body: true,
        ..cell(true, 2, 700, setting(160, 200, 120))
    };
    assert_eq!(classify(&front), ShoreClass::SandBeach);
}
