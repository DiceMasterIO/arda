//! Site readings and origin hooks for settlements (spec step 7, goal 40).
//!
//! The names themselves come from `arda-names` (see [`crate::naming`]).
//! This module says what a settlement's site is and writes the one-line
//! history hook a referee reads.

use crate::grid::Grid;
use crate::tags;

/// What a settlement's site is, most telling first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// At a ford or bridge.
    Ford,
    /// At a river mouth.
    Mouth,
    /// On a sheltered harbour.
    Haven,
    /// Beside a lake.
    Mere,
    /// On a hill.
    Hill,
    /// Beside marsh.
    Fen,
    /// In forest.
    Wood,
    /// On an open shore.
    Strand,
    /// Where rivers meet.
    Meet,
    /// Below a pass.
    Gate,
    /// At a mine.
    Delve,
    /// At a spring.
    Well,
    /// No particular site.
    Plain,
}

/// The site a settlement's tags describe, most telling first.
#[must_use]
pub fn site_of(bits: u32, mining: bool) -> Site {
    let has = |b: u32| bits & b != 0;
    if has(tags::ESTUARY) {
        Site::Mouth
    } else if has(tags::HARBOUR) {
        Site::Haven
    } else if has(tags::CONFLUENCE) {
        Site::Meet
    } else if has(tags::FORD) || has(tags::BRIDGE) {
        Site::Ford
    } else if has(tags::LAKE) {
        Site::Mere
    } else if has(tags::PASS) {
        Site::Gate
    } else if mining && has(tags::ORE) {
        Site::Delve
    } else if has(tags::DEFENSIBLE) {
        Site::Hill
    } else if has(tags::MARSH) {
        Site::Fen
    } else if has(tags::FOREST) {
        Site::Wood
    } else if has(tags::COAST) {
        Site::Strand
    } else if has(tags::SPRING) {
        Site::Well
    } else {
        Site::Plain
    }
}

/// The one-line origin hook for a settlement.
#[must_use]
pub fn history(site: Site, river: Option<&str>, peak: Option<&str>, abbey: bool) -> String {
    let on = |what: &str| match river {
        Some(r) => format!("{what} the {r}"),
        None => format!("{what} a nameless brook"),
    };
    if abbey {
        return "grew around the lands of an abbey".to_string();
    }
    match site {
        Site::Mouth => on("grew at the mouth of"),
        Site::Haven => "grew around a sheltered harbour".to_string(),
        Site::Meet => on("grew where two rivers meet on"),
        Site::Ford => on("grew at the ford of"),
        Site::Mere => "grew on the shore of a lake".to_string(),
        Site::Gate => match peak {
            Some(p) => format!("grew beneath a pass below {p}"),
            None => "grew beneath a mountain pass".to_string(),
        },
        Site::Delve => "grew around the mines in the hills".to_string(),
        Site::Hill => "grew around a hillfort on a commanding height".to_string(),
        Site::Fen => "grew on dry ground at the edge of the fen".to_string(),
        Site::Wood => "grew from a clearing in the forest".to_string(),
        Site::Strand => "grew along a fishing strand".to_string(),
        Site::Well => "grew around a spring".to_string(),
        Site::Plain => "grew among its fields".to_string(),
    }
}

/// Grid key of a cell, for seeding.
#[must_use]
pub fn cell_key(g: &Grid, i: usize) -> u64 {
    let (x, y) = g.xy(i);
    u64::try_from(y).unwrap_or(0) << 32 | u64::try_from(x).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_telling_site_wins() {
        assert_eq!(site_of(tags::ESTUARY | tags::FORD, false), Site::Mouth);
        assert_eq!(site_of(tags::ORE, false), Site::Plain);
        assert_eq!(site_of(tags::ORE, true), Site::Delve);
        assert!(history(Site::Ford, Some("Teyn"), None, false).ends_with("the Teyn"));
    }
}
