//! Land use around settlements (spec step 4; artifact "Land use around
//! settlements").
//!
//! Each settlement clears 0.8 ha of fields per person (one cell is 1 ha),
//! nearest arable cells first and open ground before woodland. Villages and
//! hamlets keep pasture beyond their fields on ground too steep or rough to
//! plough; towns keep none. The built-up footprint comes from population at
//! urban or village densities, only on ground fit to build on. Buildings
//! override fields, fields override pasture. Fields follow a three-field
//! rotation, so about a third lie fallow; floodplain grass near villages and
//! hamlets is hay meadow; hamlet footprints are farmsteads.

use crate::error::SettleError;
use crate::grid::{filled, Grid};
use crate::model::{Function, Settlement, Tier};
use crate::num::iu;
use crate::tags::{self, Sites};
use arda::Cover;
use std::collections::VecDeque;

/// Land-use codes stored per cell in `landuse.bin`. The class names are the
/// canonical land-use vocabulary (arable, pasture, orchard, woodland,
/// meadow, fallow, mill, mine_quarry, farmstead) plus `built` for the
/// footprint of villages, towns and cities.
pub mod code {
    use arda_ids::LandUse;

    /// Untouched.
    pub const NONE: u8 = LandUse::None.code();
    /// Built-up footprint of a village, town or city.
    pub const BUILT: u8 = LandUse::Built.code();
    /// Ploughed fields in crop.
    pub const ARABLE: u8 = LandUse::Field.code();
    /// Pasture.
    pub const PASTURE: u8 = LandUse::Pasture.code();
    /// Orchard (farmland).
    pub const ORCHARD: u8 = LandUse::Orchard.code();
    /// Woodland kept on slopes (coppice and timber).
    pub const WOODLAND: u8 = LandUse::Woodland.code();
    /// A water mill.
    pub const MILL: u8 = LandUse::Mill.code();
    /// A mine or quarry head.
    pub const MINE_QUARRY: u8 = LandUse::Mine.code();
    /// Hay meadow on the floodplain.
    pub const MEADOW: u8 = LandUse::Meadow.code();
    /// Fields resting in the rotation (farmland).
    pub const FALLOW: u8 = LandUse::Fallow.code();
    /// A hamlet's farmsteads.
    pub const FARMSTEAD: u8 = LandUse::Farmstead.code();

    /// Class name of a code, `none` for 0 or unknown codes.
    #[must_use]
    pub const fn name(c: u8) -> &'static str {
        match c {
            BUILT => "built",
            ARABLE => "arable",
            PASTURE => "pasture",
            ORCHARD => "orchard",
            WOODLAND => "woodland",
            MILL => "mill",
            MINE_QUARRY => "mine_quarry",
            MEADOW => "meadow",
            FALLOW => "fallow",
            FARMSTEAD => "farmstead",
            _ => "none",
        }
    }

    /// Whether a code is farmland (arable, fallow or orchard).
    #[must_use]
    pub const fn is_farmland(c: u8) -> bool {
        match LandUse::from_code(c) {
            Some(l) => l.is_farmland(),
            None => false,
        }
    }
}

/// Fields per inhabitant, tenths of a hectare (artifact: eight tenths).
pub const FIELD_TENTHS_HA: u32 = 8;

/// The land-use raster with its owners.
#[derive(Debug, Clone)]
pub struct LandUse {
    /// Code per cell (see [`code`]).
    pub codes: Vec<u8>,
    /// Owning settlement id per cell; 0 when none.
    pub owner: Vec<u32>,
}

impl LandUse {
    /// Cells of farmland (arable, fallow and orchards).
    #[must_use]
    pub fn farmland_cells(&self) -> u64 {
        self.codes.iter().filter(|&&c| code::is_farmland(c)).count() as u64
    }
}

/// Search radius in cells for fields, by tier.
fn field_radius(t: Tier) -> i64 {
    match t {
        Tier::City => 160,
        Tier::Town => 120,
        Tier::Village => 40,
        Tier::Hamlet => 25,
    }
}

/// People per hectare of built-up ground.
fn built_density(t: Tier) -> u32 {
    match t {
        Tier::City => 150,
        Tier::Town => 110,
        Tier::Village => 30,
        Tier::Hamlet => 20,
    }
}

/// Whether buildings may stand on a cell.
fn buildable(g: &Grid, i: usize) -> bool {
    g.is_land(i)
        && g.order[i] == 0
        && g.slope_md[i] <= 14_000
        && !matches!(g.cover[i], Cover::Marsh | Cover::Rock | Cover::Ice)
}

/// Allocates land use for every settlement, largest first.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn allocate(
    g: &Grid,
    sites: &Sites,
    settlements: &[Settlement],
) -> Result<LandUse, SettleError> {
    let mut lu = LandUse {
        codes: filled(g.len(), code::NONE, "land use")?,
        owner: filled(g.len(), 0_u32, "land owners")?,
    };
    let mut order: Vec<&Settlement> = settlements.iter().collect();
    order.sort_by(|a, b| b.population.cmp(&a.population).then(a.id.cmp(&b.id)));
    for s in &order {
        built(g, &mut lu, s);
    }
    for s in &order {
        fields(g, sites, &mut lu, s);
    }
    for s in order.iter().filter(|s| !s.tier.is_urban()) {
        pasture_and_wood(g, &mut lu, s);
    }
    for s in &order {
        works(g, sites, &mut lu, s);
    }
    Ok(lu)
}

fn claim(lu: &mut LandUse, i: usize, c: u8, owner: u64) {
    lu.codes[i] = c;
    lu.owner[i] = u32::try_from(owner).unwrap_or(0);
}

/// Breadth-first footprint from the centre over buildable, unclaimed ground.
fn built(g: &Grid, lu: &mut LandUse, s: &Settlement) {
    let want = (s.population / built_density(s.tier)).max(1);
    let start = s.index(g.width);
    let mut queue = VecDeque::from([start]);
    let mut seen = std::collections::BTreeSet::from([start]);
    let mut got = 0;
    while let Some(i) = queue.pop_front() {
        if got >= want || seen.len() > 200_000 {
            break;
        }
        if lu.codes[i] == code::NONE && buildable(g, i) {
            let c = if s.tier == Tier::Hamlet {
                code::FARMSTEAD
            } else {
                code::BUILT
            };
            claim(lu, i, c, s.id.get());
            got += 1;
        } else if i != start {
            continue;
        }
        for j in g.neighbours4(i) {
            if seen.insert(j) {
                queue.push_back(j);
            }
        }
    }
}

/// Unclaimed cells within `r` of the centre that satisfy `ok`, nearest first,
/// with `penalty` added to the squared distance to push some cells last.
fn nearest(
    g: &Grid,
    lu: &LandUse,
    s: &Settlement,
    r: i64,
    ok: impl Fn(usize) -> bool,
    penalty: impl Fn(usize) -> i64,
) -> Vec<usize> {
    let (cx, cy) = (i64::from(s.cell_x), i64::from(s.cell_y));
    let mut out: Vec<(i64, usize)> = Vec::new();
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            let d2 = (x - cx).pow(2) + (y - cy).pow(2);
            if d2 > r * r {
                continue;
            }
            if let Some(i) = g.at(x, y) {
                if lu.codes[i] == code::NONE && ok(i) {
                    out.push((d2 + penalty(i), i));
                }
            }
        }
    }
    out.sort_unstable();
    out.into_iter().map(|(_, i)| i).collect()
}

/// Fields at 0.8 ha a head, open ground before woodland; a tenth of village
/// and town fields on warm, sunny, gentle slopes become orchards.
fn fields(g: &Grid, sites: &Sites, lu: &mut LandUse, s: &Settlement) {
    let need = iu(i64::from(s.population * FIELD_TENTHS_HA / 10));
    let r = field_radius(s.tier);
    // Forest is cleared last: its distance is pushed beyond the whole disc.
    let cells = nearest(
        g,
        lu,
        s,
        r,
        |i| sites.tags[i] & tags::ARABLE != 0,
        |i| {
            if g.cover[i] == Cover::Forest {
                4 * r * r
            } else {
                0
            }
        },
    );
    let mut orchards = if s.tier == Tier::Hamlet { 0 } else { need / 10 };
    for &i in cells.iter().take(need) {
        let sunny = (120..=240).contains(&g.aspect_deg[i]);
        let orchard =
            orchards > 0 && sunny && g.temp_cc[i] >= 900 && (3000..=8000).contains(&g.slope_md[i]);
        if orchard {
            orchards -= 1;
        }
        // Three-field rotation: about a third of the fields rest each year.
        let fallow = crate::rng::mix(u64::try_from(i).unwrap_or(0)).is_multiple_of(3);
        let c = if orchard {
            code::ORCHARD
        } else if fallow {
            code::FALLOW
        } else {
            code::ARABLE
        };
        claim(lu, i, c, s.id.get());
    }
}

/// Hay meadow on the floodplain, pasture beyond the fields on rough
/// ground, and woodland kept on slopes.
fn pasture_and_wood(g: &Grid, lu: &mut LandUse, s: &Settlement) {
    let r = field_radius(s.tier) * 3 / 2;
    let wet_flat = |i: usize| {
        g.is_land(i)
            && g.order[i] == 0
            && g.har_dm[i] < 15
            && g.slope_md[i] <= 3000
            && matches!(g.cover[i], Cover::Grass | Cover::Scrub)
    };
    let hay = iu(i64::from(s.population * 2 / 10));
    for i in nearest(g, lu, s, r, wet_flat, |_| 0).into_iter().take(hay) {
        claim(lu, i, code::MEADOW, s.id.get());
    }
    let need = iu(i64::from(s.population * 4 / 10));
    let rough = |i: usize| {
        g.is_land(i)
            && g.order[i] == 0
            && matches!(g.cover[i], Cover::Grass | Cover::Scrub)
            && g.slope_md[i] <= 24_000
    };
    for i in nearest(g, lu, s, r, rough, |_| 0).into_iter().take(need) {
        claim(lu, i, code::PASTURE, s.id.get());
    }
    let wood = |i: usize| g.is_land(i) && g.cover[i] == Cover::Forest && g.slope_md[i] > 12_000;
    let keep = iu(i64::from(s.population * 3 / 10));
    for i in nearest(g, lu, s, r, wood, |_| 0).into_iter().take(keep) {
        claim(lu, i, code::WOODLAND, s.id.get());
    }
}

/// A mill beside the nearest sizeable stream, a mine at the nearest ore.
fn works(g: &Grid, sites: &Sites, lu: &mut LandUse, s: &Settlement) {
    if s.tier != Tier::Hamlet {
        let beside_stream = |i: usize| {
            g.is_land(i)
                && g.order[i] == 0
                && g.neighbours4(i)
                    .any(|j| g.is_watercourse(j) && g.order[j] >= 2)
        };
        let mut cells = nearest(g, lu, s, 10, beside_stream, |_| 0);
        if cells.is_empty() {
            // Fields may already hold the bank; a mill displaces one.
            cells = near_any(
                g,
                s,
                10,
                beside_stream,
                |i| code::is_farmland(lu.codes[i]),
                lu,
            );
        }
        if let Some(&i) = cells.first() {
            claim(lu, i, code::MILL, s.id.get());
        }
    }
    if s.has(Function::Mining) {
        let ore = |i: usize| sites.tags[i] & tags::ORE != 0 && g.is_land(i) && g.order[i] == 0;
        if let Some(&i) = nearest(g, lu, s, 30, ore, |_| 0).first() {
            claim(lu, i, code::MINE_QUARRY, s.id.get());
        }
    }
}

/// Like [`nearest`] but also admits cells where `reclaimable` holds.
fn near_any(
    g: &Grid,
    s: &Settlement,
    r: i64,
    ok: impl Fn(usize) -> bool,
    reclaimable: impl Fn(usize) -> bool,
    lu: &LandUse,
) -> Vec<usize> {
    let (cx, cy) = (i64::from(s.cell_x), i64::from(s.cell_y));
    let mut out: Vec<(i64, usize)> = Vec::new();
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            if let Some(i) = g.at(x, y) {
                if ok(i) && (lu.codes[i] == code::NONE || reclaimable(i)) {
                    out.push(((x - cx).pow(2) + (y - cy).pow(2), i));
                }
            }
        }
    }
    out.sort_unstable();
    out.into_iter().map(|(_, i)| i).collect()
}
