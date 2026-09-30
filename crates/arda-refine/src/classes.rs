//! Ground classes: the corner labels of the WFC tile vocabulary, the pairs
//! that may meet inside one tile, and their vocabulary ground keys
//! (`docs/goal-prompts/vocabulary.md`).

/// A natural ground class, assigned to square corners by the WFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Class {
    /// Open water (rivers, lakes, sea, marsh pools).
    Water = 0,
    /// Short grass.
    Grass = 1,
    /// Flowering meadow.
    Meadow = 2,
    /// Closed forest floor.
    ForestFloor = 3,
    /// Leaf litter at forest edges and in open woods.
    LeafLitter = 4,
    /// Heather heath.
    Heath = 5,
    /// Scrub and undergrowth.
    Scrub = 6,
    /// Moss on rock and shaded ground.
    Moss = 7,
    /// Loose scree.
    Scree = 8,
    /// Bare rock.
    Rock = 9,
    /// Cliff edge and face.
    Cliff = 10,
    /// Sand beach or bar.
    Sand = 11,
    /// Gravel bar or shingle.
    Gravel = 12,
    /// Mud bank.
    Mud = 13,
    /// Marsh.
    Marsh = 14,
    /// Reed bed.
    ReedBed = 15,
    /// Snow.
    Snow = 16,
    /// Ice.
    Ice = 17,
    /// Bare earth.
    Dirt = 18,
}

/// Number of classes.
pub const COUNT: usize = 19;

/// Every class, in discriminant order.
pub const ALL: [Class; COUNT] = [
    Class::Water,
    Class::Grass,
    Class::Meadow,
    Class::ForestFloor,
    Class::LeafLitter,
    Class::Heath,
    Class::Scrub,
    Class::Moss,
    Class::Scree,
    Class::Rock,
    Class::Cliff,
    Class::Sand,
    Class::Gravel,
    Class::Mud,
    Class::Marsh,
    Class::ReedBed,
    Class::Snow,
    Class::Ice,
    Class::Dirt,
];

/// A set of classes as a bit mask.
pub type Mask = u32;

/// Every class.
pub const ALL_MASK: Mask = (1 << COUNT) - 1;

/// Every class but water and cliff: what open ground may hold.
pub const LAND_MASK: Mask = ALL_MASK & !Class::Water.bit() & !Class::Cliff.bit();

impl Class {
    /// This class as a one-bit mask.
    #[must_use]
    pub const fn bit(self) -> Mask {
        1 << self as u8
    }

    /// Index into per-class arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The class with discriminant `i`.
    #[must_use]
    pub fn from_index(i: usize) -> Option<Self> {
        ALL.get(i).copied()
    }

    /// The vocabulary ground key for a land square of this class.
    #[must_use]
    pub const fn ground_key(self) -> &'static str {
        match self {
            Self::Water => "water_shallow",
            Self::Grass => "grass",
            Self::Meadow => "meadow",
            Self::ForestFloor => "forest_floor",
            Self::LeafLitter => "leaf_litter",
            Self::Heath => "heath",
            Self::Scrub => "scrub",
            Self::Moss => "moss",
            Self::Scree => "scree",
            Self::Rock => "rock",
            Self::Cliff => "cliff",
            Self::Sand => "sand",
            Self::Gravel => "gravel",
            Self::Mud => "mud",
            Self::Marsh => "marsh",
            Self::ReedBed => "reed_bed",
            Self::Snow => "snow",
            Self::Ice => "ice",
            Self::Dirt => "dirt",
        }
    }

    /// Tie-break rank when two classes share a square equally: the harder,
    /// more tactically significant ground wins (lower rank first).
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Cliff => 0,
            Self::Water => 1,
            Self::Rock => 2,
            Self::Scree => 3,
            Self::Ice => 4,
            Self::Snow => 5,
            Self::ReedBed => 6,
            Self::Marsh => 7,
            Self::Mud => 8,
            Self::Scrub => 9,
            Self::ForestFloor => 10,
            Self::Gravel => 11,
            Self::Sand => 12,
            Self::Heath => 13,
            Self::Moss => 14,
            Self::LeafLitter => 15,
            Self::Meadow => 16,
            Self::Dirt => 17,
            Self::Grass => 18,
        }
    }
}

use Class::{
    Cliff, Dirt, ForestFloor, Grass, Gravel, Heath, Ice, LeafLitter, Marsh, Meadow, Moss, Mud,
    ReedBed, Rock, Sand, Scree, Scrub, Snow, Water,
};

/// Class pairs that may share a tile: the natural transitions. Each pair
/// yields 14 transition tiles (every two-class corner pattern).
pub const PAIRS: [(Class, Class); 72] = [
    // Water margins: banks, beaches, shallows.
    (Water, Sand),
    (Water, Gravel),
    (Water, Mud),
    (Water, Rock),
    (Water, ReedBed),
    (Water, Marsh),
    (Water, Grass),
    (Water, Ice),
    (Water, Cliff),
    // Open grassland.
    (Grass, Meadow),
    (Grass, LeafLitter),
    (Grass, ForestFloor),
    (Grass, Heath),
    (Grass, Scrub),
    (Grass, Moss),
    (Grass, Gravel),
    (Grass, Sand),
    (Grass, Mud),
    (Grass, Marsh),
    (Grass, ReedBed),
    (Grass, Dirt),
    (Grass, Rock),
    (Grass, Scree),
    (Grass, Snow),
    (Meadow, LeafLitter),
    (Meadow, Heath),
    (Meadow, Scrub),
    (Meadow, Marsh),
    (Meadow, Dirt),
    // Woodland.
    (ForestFloor, LeafLitter),
    (ForestFloor, Moss),
    (ForestFloor, Scrub),
    (ForestFloor, Mud),
    (ForestFloor, Rock),
    (ForestFloor, Snow),
    (ForestFloor, Marsh),
    (LeafLitter, Mud),
    (LeafLitter, Scrub),
    (LeafLitter, Moss),
    (LeafLitter, Dirt),
    // Heath and scrub.
    (Heath, Scrub),
    (Heath, Moss),
    (Heath, Rock),
    (Heath, Scree),
    (Heath, Marsh),
    (Heath, Sand),
    (Scrub, Rock),
    (Scrub, Scree),
    (Scrub, Dirt),
    (Scrub, Sand),
    // Rock country.
    (Moss, Rock),
    (Moss, Scree),
    (Moss, Marsh),
    (Scree, Rock),
    (Scree, Cliff),
    (Scree, Gravel),
    (Scree, Snow),
    (Rock, Cliff),
    (Rock, Gravel),
    (Rock, Sand),
    (Rock, Snow),
    (Cliff, Snow),
    // Shores and wet ground.
    (Sand, Gravel),
    (Sand, Dirt),
    (Gravel, Mud),
    (Mud, Marsh),
    (Mud, ReedBed),
    (Marsh, ReedBed),
    // Cold and bare ground.
    (Snow, Ice),
    (Ice, Rock),
    (Gravel, Dirt),
    (Mud, Dirt),
];

const fn build_partners() -> [Mask; COUNT] {
    let mut out = [0; COUNT];
    let mut i = 0;
    while i < COUNT {
        out[i] = 1 << i;
        i += 1;
    }
    let mut p = 0;
    while p < PAIRS.len() {
        let (a, b) = PAIRS[p];
        out[a as usize] |= b.bit();
        out[b as usize] |= a.bit();
        p += 1;
    }
    out
}

/// For each class, the mask of classes it may share a tile with, itself
/// included.
pub const PARTNERS: [Mask; COUNT] = build_partners();

/// Whether two classes may share a tile.
#[must_use]
pub fn may_meet(a: Class, b: Class) -> bool {
    PARTNERS[a.index()] & b.bit() != 0
}

/// The lowest-index class in a mask.
#[must_use]
pub fn first(mask: Mask) -> Option<Class> {
    if mask == 0 {
        None
    } else {
        Class::from_index(mask.trailing_zeros() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_are_unique_and_distinct() {
        let mut seen = std::collections::BTreeSet::new();
        for &(a, b) in &PAIRS {
            assert_ne!(a, b);
            assert!(seen.insert((a.min(b), a.max(b))), "{a:?} {b:?} twice");
        }
    }

    #[test]
    fn every_class_can_reach_every_other_through_transitions() {
        let p = PARTNERS;
        let mut reach = Class::Grass.bit();
        for _ in 0..COUNT {
            for c in ALL {
                if reach & c.bit() != 0 {
                    reach |= p[c.index()];
                }
            }
        }
        assert_eq!(reach, ALL_MASK);
    }

    #[test]
    fn ground_keys_come_from_the_vocabulary() {
        let vocab = [
            "grass",
            "dirt",
            "mud",
            "sand",
            "gravel",
            "water_shallow",
            "meadow",
            "forest_floor",
            "leaf_litter",
            "heath",
            "scrub",
            "moss",
            "scree",
            "rock",
            "cliff",
            "snow",
            "ice",
            "marsh",
            "reed_bed",
        ];
        for c in ALL {
            assert!(vocab.contains(&c.ground_key()), "{:?}", c);
        }
    }
}
