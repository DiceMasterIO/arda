//! Dialects across a realm: sound changes spread over the map as isoglosses.
//!
//! Rule: each sound change holds on one side of a straight isogloss through
//! the realm. Two towns differ by every isogloss that runs between them, and
//! the chance that a line separates two points grows with their distance,
//! so neighbours sound related and distant towns differ. Changes apply in
//! isogloss order, as a history.

use crate::change::SoundChange;
use crate::language::Language;
use crate::meaning::ALL;
use crate::rng::{hash_str, Rng};
use serde::{Deserialize, Serialize};

/// Most isoglosses drawn for one realm.
const MAX_ISOGLOSSES: usize = 10;
/// Lexicon words a change must alter to count as audible.
const MIN_AFFECTED: usize = 4;

/// Integer unit directions (×1000) for isogloss normals, 16 compass points.
const DIRS: [(i64, i64); 16] = [
    (1000, 0),
    (924, 383),
    (707, 707),
    (383, 924),
    (0, 1000),
    (-383, 924),
    (-707, 707),
    (-924, 383),
    (-1000, 0),
    (-924, -383),
    (-707, -707),
    (-383, -924),
    (0, -1000),
    (383, -924),
    (707, -707),
    (924, -383),
];

/// One sound change and the line it stops at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Isogloss {
    /// The change.
    pub change: SoundChange,
    /// A point on the line.
    pub at: (i64, i64),
    /// Normal pointing into the side where the change holds (×1000).
    pub normal: (i64, i64),
}

impl Isogloss {
    /// Whether the change holds at `(x, y)`.
    #[must_use]
    pub fn holds(&self, x: i64, y: i64) -> bool {
        // i128: any i64 point and normal, deserialised or extreme, fits.
        let d = |a: i64, b: i64| i128::from(a) - i128::from(b);
        d(x, self.at.0) * i128::from(self.normal.0) + d(y, self.at.1) * i128::from(self.normal.1)
            > 0
    }
}

/// A language spread over a realm, with dialect isoglosses. Coordinates
/// are any integer unit the caller uses (cells, metres) within
/// `0..width × 0..height`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialectMap {
    /// The standard language.
    pub base: Language,
    /// Isoglosses in historical order.
    pub isoglosses: Vec<Isogloss>,
}

impl DialectMap {
    /// Draws isoglosses for `base` over a `width × height` realm.
    #[must_use]
    pub fn new(base: &Language, seed: u64, width: i64, height: i64) -> Self {
        let mut rng = Rng::new(seed, &[hash_str("dialect"), base.seed]);
        let mut candidates: Vec<SoundChange> = SoundChange::GENERAL.to_vec();
        candidates.extend(SoundChange::shifts());
        // Deterministic Fisher-Yates.
        for i in (1..candidates.len()).rev() {
            candidates.swap(i, rng.below(i + 1));
        }
        let audible = |c: SoundChange| {
            ALL.iter()
                .filter(|&&m| {
                    let w = base.root(m);
                    c.apply(w) != w
                })
                .take(MIN_AFFECTED)
                .count()
                >= MIN_AFFECTED
        };
        let mut isoglosses = Vec::new();
        for c in candidates {
            if isoglosses.len() >= MAX_ISOGLOSSES {
                break;
            }
            if !audible(c) {
                continue;
            }
            let at = (
                i64::try_from(rng.below(usize::try_from(width.max(1)).unwrap_or(1))).unwrap_or(0),
                i64::try_from(rng.below(usize::try_from(height.max(1)).unwrap_or(1))).unwrap_or(0),
            );
            let normal = DIRS[rng.below(DIRS.len())];
            isoglosses.push(Isogloss {
                change: c,
                at,
                normal,
            });
        }
        Self {
            base: base.clone(),
            isoglosses,
        }
    }

    /// Sound changes that hold at `(x, y)`.
    #[must_use]
    pub fn changes_at(&self, x: i64, y: i64) -> Vec<SoundChange> {
        self.isoglosses
            .iter()
            .filter(|i| i.holds(x, y))
            .map(|i| i.change)
            .collect()
    }

    /// The local dialect at `(x, y)`: the base language plus local changes.
    #[must_use]
    pub fn dialect_at(&self, x: i64, y: i64) -> Language {
        self.base.with_changes(&self.changes_at(x, y))
    }
}
