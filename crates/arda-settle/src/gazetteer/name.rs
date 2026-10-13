//! Naming the found features, last, from the world's shared name scope.

use super::{kind_order, min_zoom_of, rank_of, Found, GazetteerFeature};
use crate::culture::CultureMap;
use crate::error::SettleError;
use crate::grid::Grid;
use crate::naming::Naming;
use crate::output::NamedRiver;
use arda_names::{PlaceKind, PlaceSpec};

/// Rivers shorter than this are left out, metres.
const RIVER_MIN_M: u64 = 8_000;
/// Longest river path, points.
const RIVER_POINTS: usize = super::shape::MAX_PATH;

/// Names `found` in order, adds the rivers and numbers the result.
///
/// # Errors
/// [`SettleError`] when a name cannot be made unique.
pub fn name_features(
    nm: &mut Naming<'_>,
    cultures: &CultureMap,
    g: &Grid,
    found: Vec<Found>,
    rivers: &[NamedRiver],
) -> Result<Vec<GazetteerFeature>, SettleError> {
    let mut out = Vec::new();
    for f in found {
        let (cx, cy) = (f.anchor_m[0] / 100, f.anchor_m[1] / 100);
        let key = crate::names::cell_key(g, g.at(cx, cy).unwrap_or(0));
        let n = nm.name(cx, cy, &PlaceSpec::new(f.kind, key))?;
        let rank = rank_of(f.kind, f.area_ha, f.detail);
        out.push(GazetteerFeature {
            id: 0,
            kind: f.kind,
            name: n.native,
            name_gloss: n.gloss,
            culture: cultures.at(cx, cy).key().to_string(),
            anchor_m: f.anchor_m,
            path_m: f.path_m,
            extent_m: f.extent_m,
            area_ha: f.area_ha,
            length_m: f.length_m,
            rank,
            min_zoom: min_zoom_of(rank),
        });
    }
    for r in rivers
        .iter()
        .filter(|r| r.length_m >= RIVER_MIN_M && r.course_m.len() >= 2)
    {
        out.push(river(cultures, r));
    }
    out.sort_by_key(|f| {
        (
            kind_order(f.kind),
            f.rank,
            std::cmp::Reverse(f.area_ha.max(f.length_m)),
            f.anchor_m,
        )
    });
    for (k, f) in out.iter_mut().enumerate() {
        f.id = u64::try_from(k + 1).unwrap_or(u64::MAX);
    }
    Ok(out)
}

fn river(cultures: &CultureMap, r: &NamedRiver) -> GazetteerFeature {
    let n = r.course_m.len();
    let take = n.min(RIVER_POINTS);
    let path: Vec<[i64; 2]> = (0..take)
        .map(|k| r.course_m[k * (n - 1) / (take - 1).max(1)])
        .collect();
    let mut path = path;
    path.dedup();
    let xs = path.iter().map(|p| p[0]);
    let ys = path.iter().map(|p| p[1]);
    let extent_m = [
        xs.clone().min().unwrap_or(0),
        ys.clone().min().unwrap_or(0),
        xs.max().unwrap_or(0),
        ys.max().unwrap_or(0),
    ];
    let anchor_m = path[path.len() / 2];
    let rank = rank_of(PlaceKind::River, 0, u32::from(r.order));
    GazetteerFeature {
        id: 0,
        kind: PlaceKind::River,
        name: r.name.clone(),
        name_gloss: r.name_gloss.clone(),
        culture: cultures
            .at(r.mouth_m[0] / 100, r.mouth_m[1] / 100)
            .key()
            .to_string(),
        anchor_m,
        path_m: Some(path),
        extent_m,
        area_ha: 0,
        length_m: r.length_m,
        rank,
        min_zoom: min_zoom_of(rank),
    }
}
