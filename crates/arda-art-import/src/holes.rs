//! Enclosed background pockets: backdrop-coloured gaps the border flood
//! cannot reach because the subject surrounds them (sky-white between the
//! leaf clusters of a canopy, the inside of a wreath).
//!
//! Candidates are the connected regions (4-connected) of unremoved pixels
//! within the flood tolerance of the backdrop colour. A region is cleared
//! only when it is *background-like*: close to the exact backdrop colour on
//! average, flat (low luma spread) and small or thin next to the subject.
//! Large flat white areas (snow, a sheet, a whitewashed wall) stay. How
//! eager this is depends on the class and the manifest's `holes` key, see
//! [`HolePolicy`].

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use crate::manifest::HoleMode;
use crate::matte::{neighbours, Background};
use crate::naming::Target;
use crate::ops::{chan_dist, distance_to, luma};
use arda_tactical::Rgba;
use std::collections::VecDeque;

/// How pockets are treated for one asset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HolePolicy {
    /// Never clear (`holes = "keep"`, and textures).
    Off,
    /// Props and walls by default: only small, flat pockets that match the
    /// backdrop extremely closely.
    #[default]
    Strict,
    /// Vegetation by default: small or thin flat pockets near the backdrop
    /// colour (leaf gaps).
    Foliage,
    /// `holes = "clear"`: every flat pocket near the backdrop colour, at any
    /// size.
    Forced,
}

/// The thresholds a policy applies to one candidate region.
#[derive(Debug, Clone, Copy)]
struct Limits {
    /// Every pixel within this max-channel distance of the backdrop.
    worst: u8,
    /// Mean max-channel distance to the backdrop at most this.
    mean: f64,
    /// Luma standard deviation at most this.
    spread: f64,
    /// Area at most this share of the subject (`None`: any size).
    share: Option<f64>,
    /// Regions no thicker than this (Chebyshev half-width, px) may be up
    /// to four times `share`.
    thin_px: u32,
}

impl HolePolicy {
    /// The policy for an asset: the manifest's `holes` key, else by class.
    #[must_use]
    pub fn for_asset(target: &Target, mode: Option<HoleMode>) -> Self {
        match (mode, target) {
            (_, Target::Texture { .. }) | (Some(HoleMode::Keep), _) => Self::Off,
            (Some(HoleMode::Clear), _) => Self::Forced,
            (None, Target::Vegetation { .. }) => Self::Foliage,
            (None, _) => Self::Strict,
        }
    }

    fn limits(self, bg: Background) -> Option<Limits> {
        let loose = Limits {
            worst: bg.tol,
            mean: f64::from((bg.tol / 2).max(6)),
            spread: 6.0,
            share: Some(0.015),
            thin_px: 3,
        };
        match self {
            Self::Off => None,
            Self::Strict => Some(Limits {
                worst: bg.tol.min(6),
                mean: 3.0,
                spread: 2.5,
                share: Some(0.005),
                thin_px: 0,
            }),
            Self::Foliage => Some(loose),
            Self::Forced => Some(Limits {
                share: None,
                ..loose
            }),
        }
    }
}

/// One enclosed region: its area and centroid (source pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pocket {
    /// Pixels.
    pub area: usize,
    /// Centroid `(x, y)`.
    pub at: (u32, u32),
}

/// What [`clear_pockets`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Holes {
    /// Pockets added to the removed mask.
    pub cleared: Vec<Pocket>,
    /// Backdrop-like regions kept only because they are large.
    pub kept_large: Vec<Pocket>,
}

/// Adds the background-like enclosed pockets to `removed` (the flood mask,
/// which the soft matte then treats like any other backdrop, rim included).
pub fn clear_pockets(
    img: &Rgba,
    removed: &mut [bool],
    bg: Background,
    policy: HolePolicy,
) -> Holes {
    let mut out = Holes::default();
    let Some(lim) = policy.limits(bg) else {
        return out;
    };
    let (w, h) = (img.width as usize, img.height as usize);
    let subject = removed.iter().filter(|&&r| !r).count();
    let dist: Vec<u8> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| chan_dist(*p, bg.colour))
        .collect();
    let candidate: Vec<bool> = (0..w * h)
        .map(|i| !removed[i] && dist[i] <= bg.tol)
        .collect();
    let mut seen = vec![false; w * h];
    for start in 0..w * h {
        if !candidate[start] || seen[start] {
            continue;
        }
        let region = flood(start, &candidate, &mut seen, w, h);
        let pocket = centroid(&region, w);
        match judge(img, &dist, &region, (w, h), subject, lim) {
            Verdict::Clear => {
                for &i in &region {
                    removed[i] = true;
                }
                out.cleared.push(pocket);
            }
            Verdict::TooLarge => out.kept_large.push(pocket),
            Verdict::Object => {}
        }
    }
    let order = |a: &Pocket, b: &Pocket| {
        b.area
            .cmp(&a.area)
            .then(a.at.1.cmp(&b.at.1))
            .then(a.at.0.cmp(&b.at.0))
    };
    out.cleared.sort_by(order);
    out.kept_large.sort_by(order);
    out
}

enum Verdict {
    Clear,
    TooLarge,
    Object,
}

fn flood(start: usize, candidate: &[bool], seen: &mut [bool], w: usize, h: usize) -> Vec<usize> {
    let mut region = Vec::new();
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    while let Some(i) = queue.pop_front() {
        region.push(i);
        for j in neighbours(i, w, h) {
            if candidate[j] && !seen[j] {
                seen[j] = true;
                queue.push_back(j);
            }
        }
    }
    region
}

fn centroid(region: &[usize], w: usize) -> Pocket {
    let n = region.len().max(1);
    let sx: usize = region.iter().map(|i| i % w).sum();
    let sy: usize = region.iter().map(|i| i / w).sum();
    Pocket {
        area: region.len(),
        at: ((sx / n) as u32, (sy / n) as u32),
    }
}

fn judge(
    img: &Rgba,
    dist: &[u8],
    region: &[usize],
    (w, h): (usize, usize),
    subject: usize,
    lim: Limits,
) -> Verdict {
    let n = region.len() as f64;
    let worst = region.iter().map(|&i| dist[i]).max().unwrap_or(0);
    let mean = region.iter().map(|&i| f64::from(dist[i])).sum::<f64>() / n;
    let lumas: Vec<f64> = region
        .iter()
        .map(|&i| f64::from(luma(img.get((i % w) as u32, (i / w) as u32))))
        .collect();
    let lm = lumas.iter().sum::<f64>() / n;
    let spread = (lumas.iter().map(|l| (l - lm) * (l - lm)).sum::<f64>() / n).sqrt();
    if worst > lim.worst || mean > lim.mean || spread > lim.spread {
        return Verdict::Object;
    }
    let Some(share) = lim.share else {
        return Verdict::Clear;
    };
    let part = n / subject.max(1) as f64;
    if part <= share || (part <= share * 4.0 && thickness(region, w, h) <= lim.thin_px) {
        Verdict::Clear
    } else {
        Verdict::TooLarge
    }
}

/// The largest Chebyshev distance from a region pixel to the outside.
fn thickness(region: &[usize], w: usize, h: usize) -> u32 {
    let mut outside = vec![true; w * h];
    for &i in region {
        outside[i] = false;
    }
    let d = distance_to(&outside, w, h);
    region.iter().map(|&i| d[i]).max().unwrap_or(0)
}

/// Report lines: one fix listing the cleared pockets, one flag for large
/// backdrop-like regions that were kept.
pub fn report(holes: &Holes, fixes: &mut Vec<String>, flags: &mut Vec<String>) {
    const LISTED: usize = 8;
    let list = |ps: &[Pocket]| {
        let mut s: Vec<String> = ps
            .iter()
            .take(LISTED)
            .map(|p| format!("{} px at ({}, {})", p.area, p.at.0, p.at.1))
            .collect();
        if ps.len() > LISTED {
            s.push(format!("{} more", ps.len() - LISTED));
        }
        s.join(", ")
    };
    if !holes.cleared.is_empty() {
        let total: usize = holes.cleared.iter().map(|p| p.area).sum();
        let n = holes.cleared.len();
        fixes.push(format!(
            "cleared {n} enclosed background pocket{} ({total} px): {}",
            if n == 1 { "" } else { "s" },
            list(&holes.cleared)
        ));
    }
    if !holes.kept_large.is_empty() {
        flags.push(format!(
            "kept {} large enclosed backdrop-coloured region(s) as part of the object ({}); set holes = \"clear\" if they are gaps",
            holes.kept_large.len(),
            list(&holes.kept_large)
        ));
    }
}
