//! Deterministic naming languages for Arda.
//!
//! A [`Language`] is built from a seed and a culture [`Preset`]: a phoneme
//! inventory, syllable structure, phonotactics, stress, spelling and a
//! lexicon of meaningful morphemes. Names are composed from those
//! morphemes with sandhi at the joins and carry an English gloss:
//!
//! ```
//! use arda_names::{place_name, Language, Meaning, PlaceKind, PlaceSpec, Preset, SiteTag};
//!
//! let lang = Language::new(42, Preset::Heartland);
//! let spec = PlaceSpec::new(PlaceKind::Village, 7).tags(&[SiteTag::Ford]).features(&[Meaning::Oak]);
//! let name = place_name(&lang, &spec);
//! assert_eq!(name, place_name(&lang, &spec)); // deterministic
//! println!("{} ({}, \"{}\")", name.native, name.gloss, name.literal);
//! ```
//!
//! Rivers keep names from an older substrate tongue, regions and realms
//! derive from their capitals, dialects vary over a realm through a
//! [`DialectMap`], [`NameScope`] keeps names unique, and a [`Blocklist`]
//! keeps profanity and real words out. All generation is integer-only and
//! every type is serde-serialisable.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod change;
mod compose;
pub mod dialect;
pub mod error;
pub mod filter;
pub mod harmony;
mod ids;
pub mod language;
pub mod meaning;
pub mod ortho;
pub mod person;
pub mod phoneme;
pub mod phonology;
mod place;
pub mod preset;
pub mod rng;
pub mod sandhi;
pub mod scope;
pub mod tongue;
pub mod types;

pub use change::SoundChange;
pub use dialect::{DialectMap, Isogloss};
pub use error::NamesError;
pub use filter::Blocklist;
pub use language::{Language, Options};
pub use meaning::Meaning;
pub use person::{given_name, person_name, FamilyCtx, PersonName, Sex};
pub use place::place_name;
pub use preset::{FamilyStyle, Preset};
pub use scope::NameScope;
pub use tongue::Tongue;
pub use types::{Name, Part, PlaceKind, PlaceSpec, SiteTag};

impl Language {
    /// Method form of [`place_name`].
    #[must_use]
    pub fn place_name(&self, spec: &PlaceSpec) -> Name {
        place_name(self, spec)
    }

    /// Method form of [`person_name`].
    #[must_use]
    pub fn person_name(&self, sex: Sex, ctx: &FamilyCtx) -> PersonName {
        person_name(self, sex, ctx)
    }
}
