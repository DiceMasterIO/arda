//! The countryside plan over a window plus its margin: what every square is
//! (water, road, lane, compound or field) and the list of fields.

use crate::compounds::{self, Compound, CompoundKind};
use crate::fields::{kind_of, Field};
use crate::geom::{h2, Grid, Sq, CELL_SQUARES, SQUARE_M};
use crate::input::{FieldInputs, RoadClass, TerrainSample, Tier};
use crate::linear::{self, Line, RoadNet, MAX_LANE_SQ};
use crate::partition::{components, SiteIndex, TensorField, FIELD_REACH};

/// Margin around the window, in squares: every field touching the window
/// grown by the hedgerow-tree lattice lies wholly inside it.
pub const MARGIN: i64 = 2 * FIELD_REACH + 20;
/// Smallest parcel kept as a field; smaller slivers become rough corners.
pub const MIN_FIELD_SQ: usize = 320;

/// What a square is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cover {
    /// Not claimed by any site: rough grass.
    Wild,
    /// River or lake.
    Water,
    /// A road carriageway.
    Road(RoadClass),
    /// A farm lane.
    Lane,
    /// Part of compound `i`.
    Compound(u32),
    /// Part of field `i`.
    Field(u32),
    /// A sliver too small to farm: scrub.
    Rough,
    /// A settlement's built-up core, left for the town generator.
    Built,
    /// Open ground kept clear around an unwalled compound (mill yard,
    /// quarry, mine), so field walls stand off its workings.
    Apron,
}

/// A window's plan.
pub struct Plan {
    /// Seed.
    pub seed: u64,
    /// Window rectangle in global squares: x0, y0, width, height.
    pub win: (i64, i64, i64, i64),
    /// Cover over the window plus [`MARGIN`].
    pub cover: Grid<Cover>,
    /// Terrain at square centres over the same area.
    pub terrain: Grid<TerrainSample>,
    /// Fields.
    pub fields: Vec<Field>,
    /// Compounds within lane reach of the area.
    pub compounds: Vec<Compound>,
    /// Lanes from compounds to roads, indexed like `compounds`.
    pub lanes: Vec<Option<Line>>,
    /// Field sites.
    pub sites: SiteIndex,
    /// Settlement centres in world metres.
    pub settlements: Vec<[f64; 2]>,
}

impl Plan {
    /// Builds the plan of a window.
    #[must_use]
    pub fn build(inputs: &FieldInputs<'_>, win: (i64, i64, i64, i64), seed: u64) -> Self {
        let (x0, y0) = (win.0 - MARGIN, win.1 - MARGIN);
        let (w, h) = (win.2 + 2 * MARGIN, win.3 + 2 * MARGIN);
        let mut terrain = Grid::new(x0, y0, w, h, TerrainSample::default());
        for i in 0..terrain.data.len() {
            let s = Sq::new(
                x0 + i64::try_from(i).unwrap_or(0) % w,
                y0 + i64::try_from(i).unwrap_or(0) / w,
            );
            let m = s.centre_m();
            terrain.data[i] = inputs.terrain.sample(m[0], m[1]);
        }
        let mut cover = Grid::new(x0, y0, w, h, Cover::Wild);
        for (i, (c, t)) in cover.data.iter_mut().zip(&terrain.data).enumerate() {
            let wet = match inputs.barrier_water {
                Some(f) => {
                    let k = i64::try_from(i).unwrap_or(0);
                    let m = Sq::new(x0 + k % w, y0 + k / w).centre_m();
                    f(m[0], m[1])
                }
                None => t.water_depth_m > 0.0,
            };
            if wet {
                *c = Cover::Water;
            }
        }
        let net = RoadNet::new(inputs.roads);
        #[allow(clippy::cast_possible_truncation)] // a constant 960
        let reach = MAX_LANE_SQ as i64 + CELL_SQUARES;
        let cells = (
            (x0 - reach).div_euclid(CELL_SQUARES),
            (y0 - reach).div_euclid(CELL_SQUARES),
            (x0 + w + reach).div_euclid(CELL_SQUARES) + 1,
            (y0 + h + reach).div_euclid(CELL_SQUARES) + 1,
        );
        let compounds = compounds::enumerate(inputs, &net, seed, cells);
        for (i, c) in compounds.iter().enumerate() {
            let tag = Cover::Compound(u32::try_from(i).unwrap_or(u32::MAX));
            for (s, cs) in &c.squares {
                match cover.get(*s) {
                    Some(Cover::Water) if cs.water_ft == 0 => {}
                    Some(Cover::Compound(_)) | None => {}
                    Some(_) => cover.set(*s, tag),
                }
            }
        }
        let mut roads = Grid::new(x0, y0, w, h, None);
        net.rasterize(&mut roads);
        for (c, r) in cover.data.iter_mut().zip(&roads.data) {
            if let (Cover::Wild, Some(class)) = (*c, r) {
                *c = Cover::Road(*class);
            }
        }
        let lanes: Vec<Option<Line>> = compounds
            .iter()
            .map(|c| c.lane_start.and_then(|s| linear::lane(&net, s, c.lane_out)))
            .collect();
        let rect = (x0, y0, x0 + w, y0 + h);
        for lane in lanes.iter().flatten() {
            lane.for_each_square(rect, |s| {
                if cover.get(s) == Some(&Cover::Wild) {
                    cover.set(s, Cover::Lane);
                }
            });
        }
        aprons(&mut cover, &compounds);
        reserve_cores(&mut cover, inputs);
        let tf = TensorField::new(inputs.terrain, seed, rect);
        let sites = SiteIndex::new(seed, inputs.landuse, inputs.terrain, &tf, rect);
        let mut raw: Grid<Option<usize>> = Grid::new(x0, y0, w, h, None);
        for s in cover.squares().collect::<Vec<_>>() {
            if cover.get(s) == Some(&Cover::Wild) {
                raw.set(s, sites.nearest(&tf, s));
            }
        }
        let mut fields = Vec::new();
        components(&raw, |site, members| {
            if members.len() < MIN_FIELD_SQ {
                for m in members {
                    cover.set(*m, Cover::Rough);
                }
                return;
            }
            let st = &sites.sites[site];
            let (kind, crop) = kind_of(
                st.class,
                st.p,
                inputs.settlements,
                seed,
                (st.key.0 * 4 + i64::from(st.key.2), st.key.1),
            );
            let first = members[0];
            let id = h2(seed, 0x1D, first.x, first.y) & ((1 << 53) - 1);
            let mut sum = [0.0, 0.0];
            for m in members {
                let c = m.centre();
                sum[0] += c[0];
                sum[1] += c[1];
            }
            let idx = Cover::Field(u32::try_from(fields.len()).unwrap_or(u32::MAX));
            for m in members {
                cover.set(*m, idx);
            }
            let squares = u32::try_from(members.len()).unwrap_or(u32::MAX);
            let kit = crate::boundary::kit_for(inputs, kind, st.slope, squares);
            fields.push(Field {
                id,
                kind,
                crop,
                kit,
                squares,
                first,
                sum,
                site,
            });
        });
        Self {
            seed,
            win,
            cover,
            terrain,
            fields,
            compounds,
            lanes,
            sites,
            settlements: inputs.settlements.iter().map(|s| [s.x_m, s.y_m]).collect(),
        }
    }

    /// Cover of a square (`Wild` off the plan).
    #[must_use]
    pub fn at(&self, s: Sq) -> Cover {
        self.cover.get(s).copied().unwrap_or(Cover::Wild)
    }

    /// The field at a square, if any.
    #[must_use]
    pub fn field_at(&self, s: Sq) -> Option<(usize, &Field)> {
        match self.at(s) {
            Cover::Field(i) => {
                let i = usize::try_from(i).ok()?;
                self.fields.get(i).map(|f| (i, f))
            }
            _ => None,
        }
    }

    /// Whether a square lies inside the window.
    #[must_use]
    pub fn in_window(&self, s: Sq) -> bool {
        let (x0, y0, w, h) = self.win;
        s.x >= x0 && s.y >= y0 && s.x < x0 + w && s.y < y0 + h
    }
}

/// People per hectare of a built-up core, by tier (village and town
/// densities of `mockup-artifact.md`).
fn core_density(tier: Tier) -> f64 {
    match tier {
        Tier::Hamlet | Tier::Village => 60.0,
        Tier::Town | Tier::City => 150.0,
    }
}

/// Marks each settlement's built-up ground, which fields never enter: the
/// plans' own footprints when the caller knows them, else a disc sized by
/// population.
fn reserve_cores(cover: &mut Grid<Cover>, inputs: &FieldInputs<'_>) {
    if let Some(core) = inputs.cores {
        for i in 0..cover.data.len() {
            let i64w = cover.w.max(1);
            let k = i64::try_from(i).unwrap_or(0);
            let (x, y) = (cover.x0 + k % i64w, cover.y0 + k / i64w);
            if cover.data[i] == Cover::Wild && core(x, y) {
                cover.data[i] = Cover::Built;
            }
        }
        return;
    }
    for s in inputs.settlements {
        let ha = f64::from(s.population) / core_density(s.tier);
        let r = (ha * 10_000.0 / std::f64::consts::PI).sqrt() / SQUARE_M;
        let c = [s.x_m / SQUARE_M, s.y_m / SQUARE_M];
        let line = Line {
            pts: vec![c],
            half: r,
        };
        let rect = (cover.x0, cover.y0, cover.x0 + cover.w, cover.y0 + cover.h);
        line.for_each_square(rect, |q| {
            if cover.get(q) == Some(&Cover::Wild) {
                cover.set(q, Cover::Built);
            }
        });
    }
}

/// Clearance around unwalled compounds, in squares.
fn apron_radius(kind: CompoundKind) -> i64 {
    match kind {
        CompoundKind::Farmstead => 0,
        CompoundKind::Mill => 2,
        CompoundKind::Mine | CompoundKind::Quarry => 5,
    }
}

/// Marks the apron of every unwalled compound on still-unclaimed squares.
fn aprons(cover: &mut Grid<Cover>, compounds: &[Compound]) {
    for c in compounds.iter().filter(|c| !c.walled) {
        let r = apron_radius(c.kind);
        for s in c.squares.keys() {
            for dy in -r..=r {
                for dx in -r..=r {
                    let q = s.offset(dx, dy);
                    if dx * dx + dy * dy <= r * r && cover.get(q) == Some(&Cover::Wild) {
                        cover.set(q, Cover::Apron);
                    }
                }
            }
        }
    }
}
