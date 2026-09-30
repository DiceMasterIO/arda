//! Names for everything the stage makes, through `arda-names` (spec step 8,
//! `logic/15`).
//!
//! Each culture has one invented language, and every language of a world
//! shares one substrate tongue, so old river names agree across borders.
//! Each culture region spreads its language over the map with its own
//! [`DialectMap`], so neighbouring places sound alike and distant ones
//! drift apart. One [`NameScope`] covers the whole world: no two places
//! share a spelling. Places are named in a fixed order (rivers, peaks,
//! settlements by id, passes, realms, regions), which keeps the result
//! deterministic.

use crate::crossings::{Crossing, Pass};
use crate::culture::{Culture, CultureMap};
use crate::error::SettleError;
use crate::features;
use crate::grid::Grid;
use crate::model::{Function, Settlement, Tier};
use crate::names;
use crate::output::{NamedPeak, NamedRegion, NamedRiver};
use crate::realms::Realm;
use crate::rng::hash;
use crate::route;
use arda_names::{
    DialectMap, Language, Name, NameScope, Options, PlaceKind, PlaceSpec, Preset, SiteTag, Tongue,
};
use std::collections::BTreeMap;

/// Rivers of this order or more are major and keep substrate names.
const MAJOR_ORDER: u8 = 4;
/// Culture regions smaller than this many hectares get no region name.
const REGION_MIN_HA: u64 = 4096;
/// A river course keeps every this many cells.
const COURSE_STRIDE: usize = 4;

/// The language preset of a culture (keys match).
#[must_use]
pub const fn preset(c: Culture) -> Preset {
    match c {
        Culture::Heartland => Preset::Heartland,
        Culture::Highland => Preset::Highland,
        Culture::Sylvan => Preset::Sylvan,
        Culture::Coastal => Preset::Coastal,
        Culture::Southern => Preset::Southern,
        Culture::Borderland => Preset::Borderland,
    }
}

/// The naming kind of a settlement tier.
#[must_use]
pub const fn kind_of(t: Tier) -> PlaceKind {
    match t {
        Tier::Hamlet => PlaceKind::Hamlet,
        Tier::Village => PlaceKind::Village,
        Tier::Town => PlaceKind::Town,
        Tier::City => PlaceKind::City,
    }
}

/// The world's languages, dialect maps and name scope.
pub struct Naming<'a> {
    cultures: &'a CultureMap,
    seed: u64,
    width: i64,
    height: i64,
    base: BTreeMap<Culture, Language>,
    dialects: BTreeMap<u32, DialectMap>,
    scope: NameScope,
}

impl<'a> Naming<'a> {
    /// Languages for one world.
    #[must_use]
    pub fn new(seed: u64, g: &Grid, cultures: &'a CultureMap) -> Self {
        let opts = Options {
            substrate_seed: Some(hash(seed, "substrate", 0, 0)),
            blocklist: None,
        };
        let base = [
            Culture::Heartland,
            Culture::Highland,
            Culture::Sylvan,
            Culture::Coastal,
            Culture::Southern,
            Culture::Borderland,
        ]
        .into_iter()
        .map(|c| {
            let s = hash(seed, "language", c.ordinal(), 0);
            (c, Language::with_options(s, preset(c), &opts))
        })
        .collect();
        Self {
            cultures,
            seed,
            width: i64::try_from(g.width).unwrap_or(i64::MAX),
            height: i64::try_from(g.height).unwrap_or(i64::MAX),
            base,
            dialects: BTreeMap::new(),
            scope: NameScope::new(),
        }
    }

    /// The recipe of the local speech at a cell (what [`Naming::lang_at`]
    /// builds), written beside each settlement record for the NPC stage.
    #[must_use]
    pub fn tongue_at(&self, x: i64, y: i64) -> Tongue {
        let culture = self.cultures.at(x, y);
        let region = self.cultures.region_at(x, y);
        Tongue {
            preset: preset(culture),
            language_seed: hash(self.seed, "language", culture.ordinal(), 0),
            substrate_seed: hash(self.seed, "substrate", 0, 0),
            dialect_seed: hash(self.seed, "dialect", u64::from(region), 0),
            width: self.width,
            height: self.height,
            x,
            y,
        }
    }

    /// The local dialect at a cell: its culture region's language with the
    /// sound changes whose isoglosses the cell lies beyond.
    fn lang_at(&mut self, x: i64, y: i64) -> Option<Language> {
        let culture = self.cultures.at(x, y);
        let region = self.cultures.region_at(x, y);
        let base = self.base.get(&culture)?;
        let (seed, w, h) = (self.seed, self.width, self.height);
        let map = self.dialects.entry(region).or_insert_with(|| {
            DialectMap::new(base, hash(seed, "dialect", u64::from(region), 0), w, h)
        });
        Some(map.dialect_at(x, y))
    }

    /// A unique name for `spec`, spoken at cell `(x, y)`.
    fn name(&mut self, x: i64, y: i64, spec: &PlaceSpec) -> Result<Name, SettleError> {
        Ok(match self.lang_at(x, y) {
            Some(lang) => self.scope.place_name(&lang, spec)?,
            None => Name {
                native: String::new(),
                gloss: String::new(),
                literal: String::new(),
                pronunciation: String::new(),
                word: Vec::new(),
                parts: Vec::new(),
            },
        })
    }
}

/// Everything the naming stage adds besides the records it fills in place.
#[derive(Debug, Clone)]
pub struct Named {
    /// Named rivers.
    pub rivers: Vec<NamedRiver>,
    /// Named mountains.
    pub mountains: Vec<NamedPeak>,
    /// Named regions.
    pub regions: Vec<NamedRegion>,
}

/// Names rivers, peaks, settlements, crossings, passes, realms and regions.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
#[allow(clippy::too_many_arguments)]
pub fn name_world(
    g: &Grid,
    seed: u64,
    cultures: &CultureMap,
    settlements: &mut [Settlement],
    crossings: &mut [Crossing],
    crossing_cells: &[usize],
    passes: &mut [Pass],
    realms: &mut [Realm],
) -> Result<Named, SettleError> {
    let mut nm = Naming::new(seed, g, cultures);
    let (rivers, river_label) = features::rivers(g)?;
    let named_rivers: Vec<NamedRiver> = rivers
        .iter()
        .map(|r| {
            let (x, y) = g.xy(r.mouth);
            let kind = if r.order >= MAJOR_ORDER {
                PlaceKind::River
            } else {
                PlaceKind::Stream
            };
            let n = nm.name(x, y, &PlaceSpec::new(kind, u64::from(r.id)))?;
            Ok(NamedRiver {
                id: u64::from(r.id),
                name: n.native,
                name_gloss: n.gloss,
                mouth_m: [x * 100 + 50, y * 100 + 50],
                length_m: route::length_m(g, &r.cells),
                order: r.order,
                course_m: course(g, &r.cells),
            })
        })
        .collect::<Result<_, SettleError>>()?;
    let river_name = |id: u32| {
        usize::try_from(id)
            .ok()
            .and_then(|k| named_rivers.get(k.wrapping_sub(1)))
            .map(|r| r.name.clone())
    };
    let mountains: Vec<NamedPeak> = features::peaks(g)?
        .iter()
        .map(|p| {
            let (x, y) = g.xy(p.cell);
            let key = names::cell_key(g, p.cell);
            let n = nm.name(x, y, &PlaceSpec::new(PlaceKind::Mountain, key))?;
            Ok(NamedPeak {
                name: n.native,
                name_gloss: n.gloss,
                at_m: [x * 100 + 50, y * 100 + 50],
                height_m: p.height_m,
            })
        })
        .collect::<Result<_, SettleError>>()?;
    let nearest_peak = |i: usize| {
        let (x, y) = g.xy(i);
        mountains
            .iter()
            .map(|m| {
                (
                    ((m.at_m[0] / 100 - x).pow(2) + (m.at_m[1] / 100 - y).pow(2)),
                    &m.name,
                )
            })
            .filter(|(d2, _)| *d2 <= 60 * 60)
            .min()
            .map(|(_, n)| n.clone())
    };
    let mut full: BTreeMap<u64, Name> = BTreeMap::new();
    for s in settlements.iter_mut() {
        let (x, y) = (i64::from(s.cell_x), i64::from(s.cell_y));
        let tags = SiteTag::from_keys(s.site_tags.iter().map(String::as_str));
        let spec = PlaceSpec::new(kind_of(s.tier), s.id.get()).tags(&tags);
        let n = nm.name(x, y, &spec)?;
        s.name.clone_from(&n.native);
        s.name_gloss.clone_from(&n.gloss);
        s.tongue = Some(nm.tongue_at(x, y));
        let i = s.index(g.width);
        let site = names::site_of(s.tag_bits, s.has(Function::Mining));
        let river = features::river_near(g, &river_label, i, 8).and_then(river_name);
        s.history = names::history(
            site,
            river.as_deref(),
            nearest_peak(i).as_deref(),
            s.has(Function::Abbey),
        );
        full.insert(s.id.get(), n);
    }
    for (c, &cell) in crossings.iter_mut().zip(crossing_cells) {
        c.river = features::river_near(g, &river_label, cell, 3)
            .and_then(river_name)
            .unwrap_or_default();
    }
    for p in passes.iter_mut() {
        let (x, y) = g.xy(p.cell);
        let key = names::cell_key(g, p.cell);
        let n = nm.name(x, y, &PlaceSpec::new(PlaceKind::Pass, key))?;
        p.name = n.native;
        p.name_gloss = n.gloss;
    }
    for r in realms.iter_mut() {
        let Some(seat) = settlements
            .iter()
            .find(|s| s.id == arda_ids::SettlementId(r.seat))
        else {
            continue;
        };
        let (x, y) = (i64::from(seat.cell_x), i64::from(seat.cell_y));
        let mut spec = PlaceSpec::new(PlaceKind::Realm, r.id);
        if let Some(capital) = full.get(&seat.id.get()) {
            spec = spec.from_name(capital);
        }
        let n = nm.name(x, y, &spec)?;
        r.name = n.native;
        r.name_gloss = n.gloss;
        r.culture = cultures.at(x, y).key().to_string();
    }
    let regions = name_regions(&mut nm, cultures, settlements, &full)?;
    Ok(Named {
        rivers: named_rivers,
        mountains,
        regions,
    })
}

/// Regions large enough to matter take their name from their largest
/// settlement ("Oakfordshire").
fn name_regions(
    nm: &mut Naming<'_>,
    cultures: &CultureMap,
    settlements: &[Settlement],
    full: &BTreeMap<u64, Name>,
) -> Result<Vec<NamedRegion>, SettleError> {
    let mut out = Vec::new();
    for (k, &cells) in cultures.region_cells.iter().enumerate() {
        if cells < REGION_MIN_HA {
            continue;
        }
        let id = u32::try_from(k + 1).unwrap_or(0);
        let Some(at) = cultures.region.iter().position(|&r| r == id) else {
            continue;
        };
        let c = cultures
            .culture
            .get(at)
            .copied()
            .unwrap_or(Culture::Heartland);
        let capital = settlements
            .iter()
            .filter(|s| cultures.region_at(i64::from(s.cell_x), i64::from(s.cell_y)) == id)
            .max_by_key(|s| (s.population, std::cmp::Reverse(s.id)));
        let lattice = i64::try_from(crate::culture::LATTICE).unwrap_or(64);
        let wide = i64::try_from(cultures.wide.max(1)).unwrap_or(1);
        let at = i64::try_from(at).unwrap_or(0);
        let (mut x, mut y) = ((at % wide) * lattice, (at / wide) * lattice);
        let mut spec = PlaceSpec::new(PlaceKind::Region, u64::from(id));
        if let Some(s) = capital {
            (x, y) = (i64::from(s.cell_x), i64::from(s.cell_y));
            if let Some(n) = full.get(&s.id.get()) {
                spec = spec.from_name(n);
            }
        }
        let n = nm.name(x, y, &spec)?;
        out.push(NamedRegion {
            id: u64::from(id),
            name: n.native,
            name_gloss: n.gloss,
            culture: c.key().to_string(),
            land_ha: cells,
        });
    }
    Ok(out)
}

/// A river's course from its mouth upstream, every few cells, in metres.
fn course(g: &Grid, cells: &[usize]) -> Vec<[i64; 2]> {
    let mut out: Vec<[i64; 2]> = cells
        .iter()
        .step_by(COURSE_STRIDE)
        .map(|&c| {
            let (x, y) = g.xy(c);
            [x * 100 + 50, y * 100 + 50]
        })
        .collect();
    if let Some(&last) = cells.last() {
        let (x, y) = g.xy(last);
        if out.last() != Some(&[x * 100 + 50, y * 100 + 50]) {
            out.push([x * 100 + 50, y * 100 + 50]);
        }
    }
    out
}
