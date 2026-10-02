//! Settlements on relief tiles (logic/17 §land-towns): the `arda-town`
//! plans the tactical town layer composes, read square by square
//! (`arda_town::block::ground::kind`), so every building, street, yard and
//! garden is the one the tactical map draws. Plans claim squares in
//! settlement-id order, as in arda-blocks. A pixel averages a few squares
//! (up to 4 × 4) so roofs and streets resolve into a believable roofscape
//! at coarse zoom and into single buildings with lit roof pitches and
//! drop shadows (light from the north-west, logic/04) at fine zoom.

use crate::fixed::{hash2, ONE};
use crate::MidzoomError;
use arda_blocks::society::town::{Plans, Streets};
use arda_town::plan::grid::{Kind, SQUARE_M};
use arda_town::site::Tier;
use arda_town::TownPlan;
use rayon::prelude::*;
use std::sync::Arc;

/// Roof materials: thatch, red tile, brown tile, slate, shingle.
const ROOFS: [[i64; 3]; 5] = [
    [156, 136, 92],
    [142, 92, 70],
    [118, 86, 66],
    [98, 100, 106],
    [120, 104, 86],
];

/// The settlements whose plans reach a window.
pub struct Towns {
    plans: Vec<(Arc<Option<TownPlan>>, [i64; 4])>,
    streets: Streets,
}

impl std::fmt::Debug for Towns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Towns")
            .field("plans", &self.plans.len())
            .finish_non_exhaustive()
    }
}

/// What a square of a plan holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claim {
    /// Index of the plan (window order).
    pub plan: usize,
    /// Plan kind.
    pub kind: Kind,
    /// Building index + 1 (0 off buildings).
    pub building: u32,
}

impl Towns {
    /// The plans of every settlement within `margin_m` of the box
    /// `[x0, y0, x1, y1]` (metres) plus its plan radius, drawn in parallel
    /// and kept in id order.
    ///
    /// # Errors
    /// A plan could not be drawn.
    pub fn gather(
        people: &arda_people::World,
        bbox: [f64; 4],
        margin_m: f64,
    ) -> Result<Self, MidzoomError> {
        let mut ids: Vec<u64> = people
            .files
            .settlements
            .settlements
            .iter()
            .filter(|s| {
                let r = arda_people::town::plan_radius_m(s.tier) + margin_m;
                #[allow(clippy::cast_precision_loss)] // world metres
                let (x, y) = (s.x_m as f64, s.y_m as f64);
                x >= bbox[0] - r && x <= bbox[2] + r && y >= bbox[1] - r && y <= bbox[3] + r
            })
            .map(|s| s.id.get())
            .collect();
        ids.sort_unstable();
        let plans: Plans = ids
            .into_par_iter()
            .map(|id| {
                people
                    .plan(id)
                    .map(|p| (id, p))
                    .map_err(|e| MidzoomError::Society(e.to_string()))
            })
            .collect::<Result<_, _>>()?;
        let streets = Streets::of(&plans);
        let plans = plans
            .into_iter()
            .filter_map(|(_, p)| {
                let g = &p.as_ref().as_ref()?.grid;
                let b = [g.gx0, g.gy0, g.gx0 + g.w, g.gy0 + g.h];
                Some((p, b))
            })
            .collect();
        Ok(Self { plans, streets })
    }

    /// Whether a road square lies where a town's streets replace the roads.
    #[must_use]
    pub fn replaces_road(&self, gx: i64, gy: i64) -> bool {
        self.streets.replace(gx, gy)
    }

    /// The first plan (id order) occupying global square `(gx, gy)`.
    #[must_use]
    pub fn claim(&self, gx: i64, gy: i64) -> Option<Claim> {
        for (i, (p, b)) in self.plans.iter().enumerate() {
            if gx < b[0] || gy < b[1] || gx >= b[2] || gy >= b[3] {
                continue;
            }
            let Some(plan) = p.as_ref().as_ref() else {
                continue;
            };
            let Some(k) = plan.grid.gidx(gx, gy) else {
                continue;
            };
            let kind = plan.grid.kind[k];
            if matches!(kind, Kind::Open | Kind::Water) {
                continue;
            }
            return Some(Claim {
                plan: i,
                kind,
                building: plan.grid.building.get(k).copied().unwrap_or(0),
            });
        }
        None
    }

    /// Whether any plan's grid touches the box of squares.
    #[must_use]
    pub fn touches(&self, gx0: i64, gy0: i64, gx1: i64, gy1: i64) -> bool {
        self.plans
            .iter()
            .any(|(_, b)| gx1 >= b[0] && gy1 >= b[1] && gx0 < b[2] && gy0 < b[3])
    }

    fn plan(&self, i: usize) -> Option<&TownPlan> {
        self.plans.get(i).and_then(|(p, _)| p.as_ref().as_ref())
    }

    /// Paints the plans over the pixel centred at world metres `c`, of
    /// `pixel_m`. `road(gx, gy)` says whether a world road covers a square
    /// (crofts give way to it, as in the tactical composition).
    #[must_use]
    pub fn paint(
        &self,
        rgb: [u8; 3],
        c: [f64; 2],
        pixel_m: f64,
        road: &dyn Fn(i64, i64) -> bool,
    ) -> [u8; 3] {
        let sq = super::square_of;
        let half = pixel_m / 2.0;
        if !self.touches(
            sq(c[0] - half),
            sq(c[1] - half),
            sq(c[0] + half),
            sq(c[1] + half),
        ) {
            return rgb;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (pixel_m / SQUARE_M).round().clamp(1.0, 4.0) as usize;
        let mut sum = [0_i64; 3];
        let mut claimed = 0;
        #[allow(clippy::cast_precision_loss)]
        let step = pixel_m / n as f64;
        for j in 0..n {
            for i in 0..n {
                #[allow(clippy::cast_precision_loss)]
                let p = [
                    c[0] - half + (i as f64 + 0.5) * step,
                    c[1] - half + (j as f64 + 0.5) * step,
                ];
                let (gx, gy) = (sq(p[0]), sq(p[1]));
                let colour = match self.claim(gx, gy) {
                    Some(cl) if !(cl.kind == Kind::Croft && road(gx, gy)) => {
                        claimed += 1;
                        self.colour(cl, p, pixel_m, rgb)
                    }
                    _ => rgb.map(i64::from),
                };
                for k in 0..3 {
                    sum[k] += colour[k];
                }
            }
        }
        if claimed == 0 {
            return rgb;
        }
        #[allow(clippy::cast_possible_wrap)]
        let count = (n * n) as i64;
        sum.map(|v| u8::try_from((v / count).clamp(0, 255)).unwrap_or(255))
    }

    /// The colour of one claimed sample at world metres `p`.
    fn colour(&self, cl: Claim, p: [f64; 2], pixel_m: f64, ground: [u8; 3]) -> [i64; 3] {
        let g = ground.map(i64::from);
        let Some(plan) = self.plan(cl.plan) else {
            return g;
        };
        let lit = |c: [i64; 3], keep: i64| -> [i64; 3] {
            // Man-made surfaces keep a share of the terrain's light.
            std::array::from_fn(|k| (c[k] * (10 - keep) + g[k] * keep) / 10)
        };
        let paint = match cl.kind {
            Kind::Building => return self.roof(plan, cl.building, p, pixel_m),
            Kind::Street | Kind::Gate => street(plan, p),
            Kind::Square => [168, 156, 130],
            Kind::Bridge => [132, 116, 92],
            Kind::Wall => [138, 132, 118],
            Kind::Front | Kind::Plot => [140, 128, 98],
            Kind::Yard | Kind::Bailey => [132, 118, 88],
            Kind::Garden => [84, 104, 50],
            Kind::Green => [104, 134, 62],
            Kind::Churchyard => [96, 122, 60],
            Kind::Croft => [98, 120, 58],
            Kind::WaterGate => [120, 116, 104],
            Kind::Open | Kind::Water => return g,
        };
        let mut c = lit(paint, 3);
        if pixel_m <= 4.0 && self.shadowed(p) {
            c = c.map(|v| v * 62 / 100);
        }
        c
    }

    /// Whether a ground point lies in a building's shadow: the light comes
    /// from the north-west, so look that way for a building within its
    /// shadow length (two thirds of its height).
    fn shadowed(&self, p: [f64; 2]) -> bool {
        for f in [1.2, 2.4, 3.6] {
            let q = [p[0] - f, p[1] - f];
            let sq = super::square_of;
            if let Some(cl) = self.claim(sq(q[0]), sq(q[1])) {
                if cl.kind == Kind::Building {
                    let storeys = self
                        .plan(cl.plan)
                        .and_then(|pl| {
                            pl.buildings
                                .get(usize::try_from(cl.building.checked_sub(1)?).ok()?)
                        })
                        .map_or(1, |b| b.storeys.max(1));
                    if f <= 2.0 * f64::from(storeys) + 0.5 {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// A roof: material by settlement and building, pitches lit from the
    /// north-west once pixels resolve them, a darker ridge.
    fn roof(&self, plan: &TownPlan, building: u32, p: [f64; 2], pixel_m: f64) -> [i64; 3] {
        let idx = building
            .checked_sub(1)
            .and_then(|b| usize::try_from(b).ok())
            .unwrap_or(usize::MAX);
        let Some(b) = plan.buildings.get(idx) else {
            return ROOFS[0];
        };
        let h = hash2(plan.seed, 0x726f_6f66, i64::from(building), 0);
        let u = i64::try_from(h % 100).unwrap_or(0);
        let urban = matches!(plan.tier, Tier::Town | Tier::City);
        let material = match (urban, b.wealth) {
            (false, w) if w < 150 => usize::from(u >= 78) + usize::from(u >= 92) * 3,
            (false, _) => [0, 1, 4][usize::try_from(u % 3).unwrap_or(0)],
            (true, w) if w >= 170 => [3, 1, 2][usize::try_from(u % 3).unwrap_or(0)],
            (true, _) => [1, 2, 4, 1, 0][usize::try_from(u % 5).unwrap_or(0)],
        };
        let base = ROOFS[material.min(ROOFS.len() - 1)];
        // Per-building weathering, ±6 %.
        let jitter = ONE + (i64::try_from((h >> 20) & 0xFF).unwrap_or(0) - 128) * 250 / 128;
        let mut c = base.map(|v| v * jitter / ONE);
        if pixel_m <= 4.0 {
            let r = &b.rect;
            #[allow(clippy::cast_precision_loss)]
            let (cx, cy) = (
                (r.x0 + r.x1) as f64 / 2.0 * SQUARE_M,
                (r.y0 + r.y1) as f64 / 2.0 * SQUARE_M,
            );
            // The ridge runs along the longer side.
            let along_x = r.x1 - r.x0 >= r.y1 - r.y0;
            let off = if along_x { p[1] - cy } else { p[0] - cx };
            let pitch = if off < 0.0 { 112 } else { 86 };
            let ridge = off.abs() < pixel_m.max(0.6) * 0.5;
            let f = if ridge { 80 } else { pitch };
            c = c.map(|v| v * f / 100);
        }
        c
    }
}

/// Street surface: cobbles where paved, packed earth otherwise.
fn street(plan: &TownPlan, p: [f64; 2]) -> [i64; 3] {
    let sq = super::square_of;
    let paved = plan
        .grid
        .gidx(sq(p[0]), sq(p[1]))
        .and_then(|k| {
            plan.streets
                .get(usize::from(plan.grid.street[k]).checked_sub(1)?)
        })
        .is_some_and(|s| s.paved);
    if paved {
        [150, 144, 128]
    } else {
        [156, 138, 104]
    }
}

/// The paint of a claim for masks and tests: whether it is a building.
#[must_use]
pub fn is_building(c: Option<Claim>) -> bool {
    c.is_some_and(|c| c.kind == Kind::Building)
}
