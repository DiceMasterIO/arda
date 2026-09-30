//! Culture regions: contiguous zones read from the land at a 6.4 km lattice.
//!
//! Each lattice cell looks at the ~19 km around it (its 3 × 3 lattice
//! neighbourhood), so neighbouring settlements agree and cultures form
//! regions rather than a per-cell speckle. Keys match `arda-npc`'s
//! `data/content/cultures.json`.

use crate::grid::Grid;
use crate::num::{iu, ui};
use crate::tags::{self, Sites};
use arda::Cover;

/// Lattice edge in cells.
pub const LATTICE: usize = 64;

/// A culture key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Culture {
    /// Lowland farming folk (the default).
    Heartland,
    /// Hill and mountain folk.
    Highland,
    /// Forest folk.
    Sylvan,
    /// Seafaring coast folk.
    Coastal,
    /// Warm-country folk.
    Southern,
    /// Dry marchland folk.
    Borderland,
}

impl Culture {
    /// Key in `cultures.json`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Heartland => "heartland",
            Self::Highland => "highland",
            Self::Sylvan => "sylvan",
            Self::Coastal => "coastal",
            Self::Southern => "southern",
            Self::Borderland => "borderland",
        }
    }

    /// Stable small integer for seeding.
    #[must_use]
    pub const fn ordinal(self) -> u64 {
        match self {
            Self::Heartland => 0,
            Self::Highland => 1,
            Self::Sylvan => 2,
            Self::Coastal => 3,
            Self::Southern => 4,
            Self::Borderland => 5,
        }
    }
}

/// Culture and region per lattice cell.
#[derive(Debug, Clone)]
pub struct CultureMap {
    /// Lattice cells across.
    pub wide: usize,
    /// Lattice cells down.
    pub high: usize,
    /// Culture per lattice cell (meaningless where there is no land).
    pub culture: Vec<Culture>,
    /// Connected same-culture region id per lattice cell, 1-based; 0 at sea.
    pub region: Vec<u32>,
    /// Land cells per region (index = id − 1).
    pub region_cells: Vec<u64>,
}

#[derive(Default, Clone, Copy)]
struct Tally {
    land: i64,
    mountain: i64,
    coast: i64,
    forest: i64,
    temp: i64,
    moisture: i64,
}

impl CultureMap {
    /// Builds the map.
    #[must_use]
    pub fn build(g: &Grid, sites: &Sites) -> Self {
        let wide = g.width.div_ceil(LATTICE);
        let high = g.height.div_ceil(LATTICE);
        let mut tally = vec![Tally::default(); wide * high];
        // Sample every fourth cell: plenty for regional shares.
        for y in (0..g.height).step_by(4) {
            for x in (0..g.width).step_by(4) {
                let i = y * g.width + x;
                if !g.is_land(i) {
                    continue;
                }
                let t = &mut tally[(y / LATTICE) * wide + x / LATTICE];
                t.land += 1;
                t.mountain += i64::from(sites.tags[i] & tags::MOUNTAIN != 0);
                t.coast += i64::from(sites.tags[i] & tags::COAST != 0);
                t.forest += i64::from(g.cover[i] == Cover::Forest);
                t.temp += i64::from(g.temp_cc[i]);
                t.moisture += i64::from(g.moisture[i]);
            }
        }
        let mut culture = vec![Culture::Heartland; wide * high];
        for ly in 0..high {
            for lx in 0..wide {
                let mut s = Tally::default();
                for oy in -1..=1_i64 {
                    for ox in -1..=1_i64 {
                        let (nx, ny) = (ui(lx) + ox, ui(ly) + oy);
                        if nx < 0 || ny < 0 || nx >= ui(wide) || ny >= ui(high) {
                            continue;
                        }
                        let t = tally[iu(ny) * wide + iu(nx)];
                        s.land += t.land;
                        s.mountain += t.mountain;
                        s.coast += t.coast;
                        s.forest += t.forest;
                        s.temp += t.temp;
                        s.moisture += t.moisture;
                    }
                }
                culture[ly * wide + lx] = classify(s);
            }
        }
        let (region, region_cells) = regions(&culture, &tally, wide, high);
        Self {
            wide,
            high,
            culture,
            region,
            region_cells,
        }
    }

    /// Culture at a grid cell.
    #[must_use]
    pub fn at(&self, x: i64, y: i64) -> Culture {
        let lx = iu(x) / LATTICE;
        let ly = iu(y) / LATTICE;
        self.culture
            .get(ly * self.wide + lx)
            .copied()
            .unwrap_or(Culture::Heartland)
    }

    /// Region id at a grid cell.
    #[must_use]
    pub fn region_at(&self, x: i64, y: i64) -> u32 {
        let lx = iu(x) / LATTICE;
        let ly = iu(y) / LATTICE;
        self.region.get(ly * self.wide + lx).copied().unwrap_or(0)
    }
}

fn classify(s: Tally) -> Culture {
    if s.land == 0 {
        return Culture::Heartland;
    }
    let pm = |v: i64| v * 1000 / s.land;
    if pm(s.mountain) >= 450 {
        Culture::Highland
    } else if s.temp / s.land >= 1500 {
        Culture::Southern
    } else if pm(s.coast) >= 60 {
        Culture::Coastal
    } else if s.moisture / s.land < 100 {
        Culture::Borderland
    } else if pm(s.forest) >= 700 {
        Culture::Sylvan
    } else {
        Culture::Heartland
    }
}

/// Four-connected components of equal culture over lattice cells with land.
fn regions(culture: &[Culture], tally: &[Tally], wide: usize, high: usize) -> (Vec<u32>, Vec<u64>) {
    let mut region = vec![0_u32; wide * high];
    let mut sizes = Vec::new();
    for start in 0..wide * high {
        if region[start] != 0 || tally[start].land == 0 {
            continue;
        }
        sizes.push(0_u64);
        let id = u32::try_from(sizes.len()).unwrap_or(u32::MAX);
        let mut stack = vec![start];
        region[start] = id;
        while let Some(c) = stack.pop() {
            if let Some(last) = sizes.last_mut() {
                *last += u64::try_from(tally[c].land).unwrap_or(0) * 16;
            }
            let (x, y) = (c % wide, c / wide);
            let mut push = |n: usize| {
                if region[n] == 0 && tally[n].land > 0 && culture[n] == culture[c] {
                    region[n] = id;
                    stack.push(n);
                }
            };
            if x > 0 {
                push(c - 1);
            }
            if x + 1 < wide {
                push(c + 1);
            }
            if y > 0 {
                push(c - wide);
            }
            if y + 1 < high {
                push(c + wide);
            }
        }
    }
    (region, sizes)
}
