//! The town fabric by WFC at 5-ft scale (goal 44), inside the plan's
//! constraints (logic/10): the plan fixes streets, plots, footprints,
//! doors and ids; WFC fills interiors (rooms, doorways, furniture), wall
//! kit pieces, and the streets, squares, yards and crofts around them.
//!
//! [`TownFill`] is the seam between strategies: WFC is the default and the
//! rule programmes of `block::interior` and `block::exterior` stay as the
//! alternative and as the relaxed fill after bounded retries (goal 47).
//! Every problem is keyed by plan objects or global chunks, never by the
//! window, so blocks stay pixel- and data-exact at seams (goal 42).

pub mod access;
pub mod assign;
pub mod catalogue;
pub mod furnish;
pub mod indoor;
pub mod outdoor;
pub mod outdoor_vocab;
pub mod piece;
pub mod programme;
pub mod programmes;
pub mod rooms;
pub mod rows;
pub mod shell;

use serde::{Deserialize, Serialize};

/// How the town fabric inside the plan is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TownFill {
    /// Wave-function collapse over typed tile sets (default).
    #[default]
    Wfc,
    /// The rule programmes (also the WFC's relaxed fill).
    Rules,
}

/// A problem that fell back to the relaxed fill, for review (goal 47).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Relaxed {
    /// A building interior (plan id).
    Interior {
        /// Building id, as a string (I5).
        #[serde(with = "crate::ids")]
        building: u64,
    },
    /// An outdoor chunk (global chunk coordinates).
    Outdoor {
        /// Chunk column.
        cx: i64,
        /// Chunk row.
        cy: i64,
    },
}

/// Counts of WFC problems and relaxed fills, for review and the gates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// Interiors solved by WFC.
    pub interiors: usize,
    /// Interiors that used the relaxed fill.
    pub interiors_relaxed: usize,
    /// Relaxed interiors by building function key.
    pub relaxed_by_function: std::collections::BTreeMap<String, usize>,
    /// Outdoor chunks solved by WFC.
    pub chunks: usize,
    /// Outdoor chunks that used the relaxed fill.
    pub chunks_relaxed: usize,
}

impl Report {
    /// Problems in all.
    #[must_use]
    pub const fn problems(&self) -> usize {
        self.interiors + self.chunks
    }

    /// Relaxed fills in all.
    #[must_use]
    pub const fn relaxed(&self) -> usize {
        self.interiors_relaxed + self.chunks_relaxed
    }

    /// Adds another report.
    pub fn add(&mut self, o: &Self) {
        self.interiors += o.interiors;
        self.interiors_relaxed += o.interiors_relaxed;
        self.chunks += o.chunks;
        self.chunks_relaxed += o.chunks_relaxed;
        for (k, v) in &o.relaxed_by_function {
            *self.relaxed_by_function.entry(k.clone()).or_default() += v;
        }
    }
}

/// Solves every WFC problem of a plan and counts the relaxed fills.
#[must_use]
pub fn report(plan: &crate::plan::TownPlan) -> Report {
    let mut r = Report::default();
    for b in &plan.buildings {
        match indoor::solve(plan, b) {
            Ok(None) => {}
            Ok(Some(_)) => r.interiors += 1,
            Err(_) => {
                r.interiors += 1;
                r.interiors_relaxed += 1;
                *r.relaxed_by_function
                    .entry(b.function.key().to_string())
                    .or_default() += 1;
            }
        }
    }
    let site = outdoor::Site::of(plan);
    let (gx0, gy0) = plan.origin();
    let (w, h) = plan.size();
    for (cx, cy) in outdoor::chunks_near(gx0 + 3, gy0 + 3, gx0 + w - 3, gy0 + h - 3) {
        let ch = outdoor::chunk(plan, &site, cx, cy);
        if ch.any {
            r.chunks += 1;
            r.chunks_relaxed += usize::from(ch.relaxed);
        }
    }
    r
}
