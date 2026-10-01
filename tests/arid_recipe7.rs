//! Recipe 7 arid basins end to end (goal 12; logic/02 §world-water arid
//! basins): MICRO seed 74 has an audited rift basin. In the subtropical
//! belt (15-35°) its lake is terminal and saline and stands on a salt pan;
//! at the default 35-55° the same basin fills and spills, so no lake is
//! terminal there.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{FineDeliveryLimits, GenerateConfig, LatitudeBand, World};
use arda_core::water::{LakeOrigin, PanKind};
use std::path::PathBuf;

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("arda-arid-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `(arid terminal lakes, other terminal lakes, crust cells, mudflat cells)`
/// over every area of MICRO seed 74 at `band`, recipe 7.
fn survey(band: LatitudeBand, tag: &str) -> (usize, usize, usize, usize) {
    let tmp = TempDir::new(tag);
    let dir = tmp.0.join("world");
    let config = GenerateConfig::new(GenerateConfig::MICRO.size_km(), band, 15).unwrap();
    arda::generate_from_fine_recipe(74, config, &dir, FineDeliveryLimits::default(), 7)
        .expect("generation");
    let world = World::load(&dir).expect("load");
    let m = world.manifest();
    let (mut arid, mut other, mut crust, mut mud) = (0, 0, 0, 0);
    for ay in 0..m.areas_high {
        for ax in 0..m.areas_wide {
            let area = world.read_area(ax, ay).unwrap();
            let water = area.water().expect("recipe 7 stores water forms");
            for (form, lake) in water.lakes.iter().zip(area.lakes()) {
                assert_eq!(form.saline, form.terminal, "every terminal lake is saline");
                match (form.origin, form.terminal) {
                    (LakeOrigin::AridTerminal, true) => arid += lake.cells.len(),
                    (LakeOrigin::AridTerminal, false) => panic!("an open arid lake"),
                    (LakeOrigin::Tectonic, true) => other += lake.cells.len(),
                    _ => {}
                }
            }
            for run in &water.pans {
                let c = arda_core::CellCoord::new(run.x0, run.y).unwrap();
                assert_eq!(
                    area.cells().get(c).terrain,
                    arda::TerrainKind::Land,
                    "a pan is dry land"
                );
                match run.kind {
                    PanKind::SaltCrust => crust += usize::from(run.len),
                    PanKind::Mudflat => mud += usize::from(run.len),
                }
            }
        }
    }
    (arid, other, crust, mud)
}

#[test]
fn a_subtropical_rift_basin_holds_a_terminal_salt_lake_on_a_pan() {
    let (arid, other, crust, mud) = survey(LatitudeBand::new(15, 35), "dry");
    assert!(
        arid >= 1_000,
        "an arid terminal lake of at least 10 km²: {arid}"
    );
    assert_eq!(other, 0, "the rift lake is the arid terminal one");
    assert!(
        crust >= 1_000 && mud >= 200,
        "salt pan {crust} crust, {mud} mudflat"
    );
}

#[test]
fn the_same_basin_at_temperate_latitudes_spills() {
    let (arid, other, crust, mud) = survey(LatitudeBand::new(35, 55), "wet");
    assert_eq!((arid, other, crust, mud), (0, 0, 0, 0));
}
