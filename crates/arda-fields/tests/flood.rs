//! Review 3.5-14 (fields performance): a window's plan locates only the
//! fields the window can see, and they are exactly the fields a full
//! labelling of the plan's margin gives.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    missing_docs
)]

use arda_fields::geom::{Grid, Sq};
use arda_fields::partition::components;
use arda_fields::plan::{Cover, Plan};
use arda_fields::synthetic;

#[test]
fn the_plan_floods_only_the_fields_the_window_sees_and_they_are_whole() {
    for s in synthetic::all() {
        let inputs = s.inputs();
        let x0 = (s.origin_m[0] / arda_fields::geom::SQUARE_M).round() as i64;
        let y0 = (s.origin_m[1] / arda_fields::geom::SQUARE_M).round() as i64;
        // A 64-square cell in the middle of the scenario, as a block asks.
        let win = (
            x0 + i64::from(s.w) / 2 - 32,
            y0 + i64::from(s.h) / 2 - 32,
            64,
            64,
        );
        let plan = Plan::build(&inputs, win, 42);
        let area = usize::try_from(plan.cover.w * plan.cover.h).unwrap();
        assert!(
            plan.located * 3 < area,
            "{}: located {} of {area} squares",
            s.name,
            plan.located
        );
        // A full labelling of every open square the plan left open or
        // made a field or rough, compared on the fields the plan kept.
        let mut raw: Grid<Option<usize>> = Grid::new(
            plan.cover.x0,
            plan.cover.y0,
            plan.cover.w,
            plan.cover.h,
            None,
        );
        for q in plan.cover.squares().collect::<Vec<_>>() {
            if matches!(
                plan.cover.get(q),
                Some(Cover::Wild | Cover::Rough | Cover::Field(_))
            ) {
                raw.set(q, plan.partition.locate(q.centre()));
            }
        }
        let mut full: Vec<(usize, Vec<Sq>)> = Vec::new();
        components(&raw, |site, m| full.push((site, m.to_vec())));
        for f in &plan.fields {
            let whole = full
                .iter()
                .find(|(_, m)| m.first() == Some(&f.first))
                .expect("a field the full labelling lacks");
            assert_eq!(whole.0, f.site, "{}: field {}", s.name, f.id);
            assert_eq!(
                usize::try_from(f.squares).unwrap(),
                whole.1.len(),
                "{}: field {} is cut",
                s.name,
                f.id
            );
        }
    }
}
