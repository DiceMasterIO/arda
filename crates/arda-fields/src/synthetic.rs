//! Synthetic countryside scenarios for the example and the tests: analytic
//! terrain with an optional river, a land-use raster and a few roads.

use crate::geom::{h2, seg_dist2, u01};
use crate::input::{
    FieldInputs, LandUse, LandUseGrid, Region, Road, RoadClass, Settlement, Terrain, TerrainSample,
    Tier,
};
use crate::partition::gradient;
use arda_tactical::noise::fbm;

/// A river for synthetic terrain.
#[derive(Debug, Clone, PartialEq)]
pub struct River {
    /// Centre line in world metres, upstream first.
    pub pts: Vec<[f64; 2]>,
    /// Half-width in metres.
    pub half_m: f64,
    /// Depth at the centre line in metres.
    pub depth_m: f64,
    /// Water surface at the first point, metres.
    pub surface_m: f64,
    /// Fall of the surface per metre downstream.
    pub fall: f64,
}

/// Rotated fractal relief plus a tilt, a valley and a river.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthTerrain {
    /// Noise seed.
    pub seed: u64,
    /// Mean height, metres.
    pub base_m: f64,
    /// Peak-to-peak relief, metres.
    pub relief_m: f64,
    /// Relief feature size, metres.
    pub scale_m: f64,
    /// Regional tilt, metres per metre east and south.
    pub tilt: [f64; 2],
    /// Optional river.
    pub river: Option<River>,
}

impl SynthTerrain {
    fn relief(&self, x: f64, y: f64) -> f64 {
        #[allow(clippy::cast_possible_truncation)] // noise input precision
        let n = fbm(
            self.seed,
            (x / self.scale_m) as f32,
            (y / self.scale_m) as f32,
            4,
            None,
        );
        self.base_m + self.tilt[0] * x + self.tilt[1] * y + self.relief_m * (f64::from(n) - 0.5)
    }
}

impl Terrain for SynthTerrain {
    fn sample(&self, x: f64, y: f64) -> TerrainSample {
        let land = self.relief(x, y);
        let Some(r) = &self.river else {
            return TerrainSample {
                height_m: land,
                water_depth_m: 0.0,
            };
        };
        let (mut best, mut along, mut run) = (f64::MAX, 0.0, 0.0);
        for w in r.pts.windows(2) {
            let (d2, t) = seg_dist2([x, y], w[0], w[1]);
            let len = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            if d2 < best {
                best = d2;
                along = run + t * len;
            }
            run += len;
        }
        let d = best.sqrt();
        let surface = r.surface_m - r.fall * along;
        if d < r.half_m {
            let k = 1.0 - (d / r.half_m).powi(2);
            return TerrainSample {
                height_m: surface,
                water_depth_m: (r.depth_m * k).max(0.25),
            };
        }
        // The valley floor rises gently from the banks to the open land.
        let valley = 120.0;
        let t = ((d - r.half_m) / valley).clamp(0.0, 1.0);
        let s = t * t * (3.0 - 2.0 * t);
        let floor = surface + 0.8 + 2.0 * t;
        TerrainSample {
            height_m: floor + (land - floor).max(0.0) * s,
            water_depth_m: 0.0,
        }
    }
}

/// A synthetic scenario.
#[derive(Debug, Clone)]
pub struct Scenario {
    /// Name, used for output files.
    pub name: &'static str,
    /// Land use.
    pub landuse: LandUseGrid,
    /// Terrain.
    pub terrain: SynthTerrain,
    /// Settlements.
    pub settlements: Vec<Settlement>,
    /// Roads.
    pub roads: Vec<Road>,
    /// Culture.
    pub culture: String,
    /// Region.
    pub region: Region,
    /// Wealth.
    pub wealth: u8,
    /// Window origin in metres.
    pub origin_m: [f64; 2],
    /// Window width in squares.
    pub w: u32,
    /// Window height in squares.
    pub h: u32,
}

impl Scenario {
    /// Borrowed inputs for [`crate::fields_window`].
    #[must_use]
    pub fn inputs(&self) -> FieldInputs<'_> {
        FieldInputs {
            landuse: &self.landuse,
            terrain: &self.terrain,
            culture: &self.culture,
            region: self.region,
            wealth: self.wealth,
            settlements: &self.settlements,
            roads: &self.roads,
            cores: None,
            barrier_water: None,
        }
    }
}

/// Names of the built-in scenarios.
pub const NAMES: [&str; 5] = [
    "village_strips",
    "hedge_country",
    "orchard_farmstead",
    "watermill",
    "quarry",
];

/// A scenario by name.
#[must_use]
pub fn by_name(name: &str) -> Option<Scenario> {
    match name {
        "village_strips" => Some(village_strips()),
        "hedge_country" => Some(hedge_country()),
        "orchard_farmstead" => Some(orchard_farmstead()),
        "watermill" => Some(watermill()),
        "quarry" => Some(quarry()),
        _ => None,
    }
}

/// Every built-in scenario.
#[must_use]
pub fn all() -> Vec<Scenario> {
    NAMES.iter().filter_map(|n| by_name(n)).collect()
}

/// A quadratic Bézier road sampled every ~20 m.
#[must_use]
pub fn curve(class: RoadClass, a: [f64; 2], c: [f64; 2], b: [f64; 2]) -> Road {
    let n = 60;
    let points = (0..=n)
        .map(|i| {
            let t = f64::from(i) / f64::from(n);
            let u = 1.0 - t;
            [
                u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
            ]
        })
        .collect();
    Road { class, points }
}

/// A raster over cells `x0..x0+n × y0..y0+n` filled by `f`.
fn raster(x0: i64, y0: i64, n: u32, f: impl Fn(i64, i64) -> Option<LandUse>) -> LandUseGrid {
    let mut g = LandUseGrid::filled((x0, y0), n, n, None);
    for cy in y0..y0 + i64::from(n) {
        for cx in x0..x0 + i64::from(n) {
            g.set(cx, cy, f(cx, cy));
        }
    }
    g
}

/// A weighted pick of a land use per cell.
fn pick(seed: u64, cx: i64, cy: i64, table: &[(Option<LandUse>, f64)]) -> Option<LandUse> {
    let total: f64 = table.iter().map(|t| t.1).sum();
    let mut u = u01(h2(seed, 0xCE11, cx, cy)) * total;
    for (class, w) in table {
        if u < *w {
            return *class;
        }
        u -= w;
    }
    None
}

/// Slope at the centre of a 100 m cell.
fn cell_slope(t: &dyn Terrain, cx: i64, cy: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)] // cell coordinates
    let g = gradient(t, [cx as f64 * 100.0 + 50.0, cy as f64 * 100.0 + 50.0]);
    (g[0] * g[0] + g[1] * g[1]).sqrt()
}

fn terrain(seed: u64, relief_m: f64, scale_m: f64, river: Option<River>) -> SynthTerrain {
    SynthTerrain {
        seed,
        base_m: 80.0,
        relief_m,
        scale_m,
        tilt: [0.004, -0.003],
        river,
    }
}

/// A village with open-field strips on the plain around it.
#[must_use]
pub fn village_strips() -> Scenario {
    let terrain = terrain(11, 10.0, 900.0, None);
    let landuse = raster(30, 30, 40, |cx, cy| {
        let d2 = (cx - 50).pow(2) + (cy - 50).pow(2);
        if d2 <= 1 {
            None
        } else if d2 <= 90 {
            pick(
                3,
                cx,
                cy,
                &[
                    (Some(LandUse::Arable), 0.86),
                    (Some(LandUse::Meadow), 0.08),
                    (Some(LandUse::Pasture), 0.06),
                ],
            )
        } else {
            pick(
                4,
                cx,
                cy,
                &[
                    (Some(LandUse::Pasture), 0.5),
                    (Some(LandUse::Woodland), 0.3),
                    (None, 0.2),
                ],
            )
        }
    });
    Scenario {
        name: "village_strips",
        landuse,
        terrain,
        settlements: vec![Settlement {
            x_m: 5050.0,
            y_m: 5050.0,
            tier: Tier::Village,
            population: 380,
        }],
        roads: vec![
            curve(
                RoadClass::Road,
                [3000.0, 5080.0],
                [5000.0, 4990.0],
                [7000.0, 5140.0],
            ),
            curve(
                RoadClass::Track,
                [5060.0, 5040.0],
                [5250.0, 4800.0],
                [5500.0, 3500.0],
            ),
        ],
        culture: "human".into(),
        region: Region::Lowland,
        wealth: 90,
        origin_m: [5250.0, 4700.0],
        w: 256,
        h: 256,
    }
}

/// Ancient enclosed countryside: small hedged fields, pasture, woods on slopes.
#[must_use]
pub fn hedge_country() -> Scenario {
    let terrain = terrain(23, 28.0, 520.0, None);
    let t2 = terrain.clone();
    let landuse = raster(0, 0, 40, move |cx, cy| {
        if cell_slope(&t2, cx, cy) > 0.08 {
            return Some(LandUse::Woodland);
        }
        pick(
            7,
            cx,
            cy,
            &[
                (Some(LandUse::Arable), 0.38),
                (Some(LandUse::Pasture), 0.3),
                (Some(LandUse::Meadow), 0.14),
                (Some(LandUse::Fallow), 0.08),
                (Some(LandUse::Woodland), 0.06),
                (Some(LandUse::Orchard), 0.04),
            ],
        )
    });
    let mut landuse = landuse;
    landuse.set(22, 21, Some(LandUse::Farmstead));
    Scenario {
        name: "hedge_country",
        landuse,
        terrain,
        settlements: vec![Settlement {
            x_m: 1500.0,
            y_m: 2600.0,
            tier: Tier::Hamlet,
            population: 60,
        }],
        roads: vec![
            curve(
                RoadClass::Road,
                [1000.0, 2400.0],
                [2100.0, 1900.0],
                [3200.0, 2300.0],
            ),
            curve(
                RoadClass::Track,
                [2150.0, 2020.0],
                [2250.0, 2300.0],
                [2150.0, 3200.0],
            ),
        ],
        culture: "human".into(),
        region: Region::Lowland,
        wealth: 110,
        origin_m: [2000.0, 2000.0],
        w: 256,
        h: 256,
    }
}

/// A farmstead with its orchard, paddocks and ploughland.
#[must_use]
pub fn orchard_farmstead() -> Scenario {
    let terrain = terrain(31, 14.0, 700.0, None);
    let landuse = raster(0, 0, 40, |cx, cy| match (cx - 20, cy - 20) {
        (0, 0) => Some(LandUse::Farmstead),
        (1, 0) | (1, -1) => Some(LandUse::Orchard),
        (-1, 0) | (0, 1) => Some(LandUse::Pasture),
        (-1, 1) => Some(LandUse::Meadow),
        _ => pick(
            9,
            cx,
            cy,
            &[
                (Some(LandUse::Arable), 0.55),
                (Some(LandUse::Pasture), 0.3),
                (Some(LandUse::Woodland), 0.15),
            ],
        ),
    });
    Scenario {
        name: "orchard_farmstead",
        landuse,
        terrain,
        settlements: vec![],
        roads: vec![curve(
            RoadClass::Road,
            [1500.0, 2180.0],
            [2050.0, 2130.0],
            [2700.0, 2250.0],
        )],
        culture: "human".into(),
        region: Region::Mixed,
        wealth: 140,
        origin_m: [1900.0, 1900.0],
        w: 192,
        h: 192,
    }
}

/// A watermill on a river among water meadows.
#[must_use]
pub fn watermill() -> Scenario {
    let river = River {
        pts: vec![
            [1500.0, 1700.0],
            [1950.0, 2020.0],
            [2150.0, 2090.0],
            [2700.0, 2150.0],
        ],
        half_m: 9.0,
        depth_m: 1.6,
        surface_m: 70.0,
        fall: 0.002,
    };
    let terrain = terrain(41, 12.0, 700.0, Some(river));
    let landuse = raster(0, 0, 40, |cx, cy| match (cx, cy) {
        (20, 20) => Some(LandUse::Mill),
        (19..=21, 20..=21) => Some(LandUse::Meadow),
        _ if cy >= 21 => pick(
            12,
            cx,
            cy,
            &[
                (Some(LandUse::Meadow), 0.4),
                (Some(LandUse::Pasture), 0.4),
                (Some(LandUse::Woodland), 0.2),
            ],
        ),
        _ => pick(
            13,
            cx,
            cy,
            &[
                (Some(LandUse::Arable), 0.5),
                (Some(LandUse::Pasture), 0.3),
                (Some(LandUse::Fallow), 0.1),
                (Some(LandUse::Woodland), 0.1),
            ],
        ),
    });
    Scenario {
        name: "watermill",
        landuse,
        terrain,
        settlements: vec![],
        roads: vec![curve(
            RoadClass::Road,
            [1600.0, 1800.0],
            [2020.0, 2000.0],
            [2500.0, 1700.0],
        )],
        culture: "human".into(),
        region: Region::Lowland,
        wealth: 120,
        origin_m: [1900.0, 1860.0],
        w: 192,
        h: 192,
    }
}

/// A hillside quarry and a mine adit in walled upland pasture.
#[must_use]
pub fn quarry() -> Scenario {
    let terrain = SynthTerrain {
        seed: 53,
        base_m: 320.0,
        relief_m: 40.0,
        scale_m: 450.0,
        tilt: [0.0, -0.09],
        river: None,
    };
    let landuse = raster(0, 0, 40, |cx, cy| match (cx, cy) {
        (20, 20) | (22, 19) => Some(LandUse::MineQuarry),
        _ if cy <= 19 => pick(17, cx, cy, &[(None, 0.6), (Some(LandUse::Pasture), 0.4)]),
        _ => pick(
            18,
            cx,
            cy,
            &[
                (Some(LandUse::Pasture), 0.55),
                (Some(LandUse::Meadow), 0.2),
                (None, 0.15),
                (Some(LandUse::Woodland), 0.1),
            ],
        ),
    });
    Scenario {
        name: "quarry",
        landuse,
        terrain,
        settlements: vec![],
        roads: vec![curve(
            RoadClass::Track,
            [1500.0, 2300.0],
            [2100.0, 2180.0],
            [2800.0, 2250.0],
        )],
        culture: "human".into(),
        region: Region::Upland,
        wealth: 70,
        origin_m: [1900.0, 1850.0],
        w: 256,
        h: 256,
    }
}
