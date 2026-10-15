//! Fallow is drawn from the world seed and the cell's position, not the
//! grid's flat index (review 3.3-15; logic/08 §landuse).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::landuse::{self, code};
use arda_settle::{run, synthetic, tags, PlaceParams};

#[test]
fn fallow_follows_the_seed_and_the_cell_not_the_index() {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let g = synthetic::landscape(params.seed).unwrap();
    let soc = run(&g, params).unwrap();
    let sites = tags::compute(&g, params.seed).unwrap();
    let a = landuse::allocate(&g, &sites, &soc.settlements, 42).unwrap();
    let b = landuse::allocate(&g, &sites, &soc.settlements, 43).unwrap();
    let farmed = |c: u8| c == code::ARABLE || c == code::FALLOW;
    let (mut fields, mut fallow, mut differ) = (0, 0, 0);
    for i in 0..g.len() {
        assert_eq!(farmed(a.codes[i]), farmed(b.codes[i]), "same fields");
        if a.codes[i] == code::ARABLE || a.codes[i] == code::FALLOW {
            fields += 1;
            let (x, y) = g.xy(i);
            let want = landuse::fallow(42, x, y);
            assert_eq!(a.codes[i] == code::FALLOW, want, "cell {x},{y}");
            fallow += usize::from(want);
            differ += usize::from(a.codes[i] != b.codes[i]);
        }
    }
    assert!(fields > 1000);
    assert!(
        (fallow * 100 / fields).abs_diff(33) <= 3,
        "{fallow} of {fields}"
    );
    assert!(differ * 4 > fields, "another seed rests other fields");
}
