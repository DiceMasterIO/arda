//! Review round 2 #38: `FieldRecord.complete` holds for open fields (strips)
//! that lie wholly inside the window, not only for enclosed ones.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_fields::generate;
use arda_fields::synthetic;
use std::collections::BTreeMap;

#[test]
fn open_fields_wholly_inside_the_window_are_complete() {
    let s = synthetic::village_strips();
    let win = generate(&s.inputs(), s.origin_m, s.w, s.h, 42).unwrap();
    let mut in_window: BTreeMap<&str, u32> = BTreeMap::new();
    for q in &win.sidecar.squares {
        if let Some(id) = &q.field {
            *in_window.entry(id.as_str()).or_insert(0) += 1;
        }
    }
    let side = win.sidecar.square_m * win.sidecar.square_m / 10_000.0;
    let mut open_complete = 0;
    for f in win.sidecar.fields.iter().filter(|f| !f.enclosed) {
        let here = f64::from(in_window[f.id.as_str()]) * side;
        let whole = (here - f.hectares).abs() < 1e-9;
        assert_eq!(
            f.complete, whole,
            "field {} ({here} of {} ha)",
            f.id, f.hectares
        );
        open_complete += usize::from(f.complete);
    }
    assert!(open_complete > 0, "some strips lie wholly in the window");
}
