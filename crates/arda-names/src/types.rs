//! Public input and output types: what to name, and the name produced.

use crate::meaning::Meaning;
use crate::phoneme::Ph;
use serde::{Deserialize, Serialize};

/// What kind of place is being named.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    /// A handful of farms.
    Hamlet,
    /// A village.
    #[default]
    Village,
    /// A market town.
    Town,
    /// A city.
    City,
    /// A fortress or castle.
    Fort,
    /// An abbey, temple or shrine.
    Abbey,
    /// A port.
    Port,
    /// A mining settlement.
    Mine,
    /// A major river (named in the old substrate tongue).
    River,
    /// A brook or minor stream (named in the living tongue).
    Stream,
    /// A lake.
    Lake,
    /// A mountain or range.
    Mountain,
    /// A hill.
    Hill,
    /// A forest.
    Forest,
    /// A marsh or fen.
    Marsh,
    /// A mountain pass.
    Pass,
    /// A bay.
    Bay,
    /// An island.
    Island,
    /// A cape or headland.
    Cape,
    /// A valley.
    Vale,
    /// A region, derived from its capital (`PlaceSpec::from`) or its features.
    Region,
    /// A realm, derived from its capital or region (`PlaceSpec::from`).
    Realm,
}

impl PlaceKind {
    /// Parses a settlement tier or feature key (`"hamlet"`, `"river"`, ...).
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        use PlaceKind as K;
        Some(match key.trim().to_ascii_lowercase().as_str() {
            "hamlet" => K::Hamlet,
            "village" => K::Village,
            "town" | "market_town" => K::Town,
            "city" => K::City,
            "fort" | "fortress" | "castle" => K::Fort,
            "abbey" | "temple" | "monastery" => K::Abbey,
            "port" => K::Port,
            "mine" | "mining_village" => K::Mine,
            "river" => K::River,
            "stream" | "brook" => K::Stream,
            "lake" => K::Lake,
            "mountain" | "range" => K::Mountain,
            "hill" => K::Hill,
            "forest" | "wood" => K::Forest,
            "marsh" | "fen" => K::Marsh,
            "pass" => K::Pass,
            "bay" => K::Bay,
            "island" => K::Island,
            "cape" | "headland" => K::Cape,
            "vale" | "valley" => K::Vale,
            "region" => K::Region,
            "realm" => K::Realm,
            _ => return None,
        })
    }

    /// Settlements, as opposed to natural features and polities.
    #[must_use]
    pub fn is_settlement(self) -> bool {
        use PlaceKind as K;
        matches!(
            self,
            K::Hamlet | K::Village | K::Town | K::City | K::Fort | K::Abbey | K::Port | K::Mine
        )
    }
}

/// A site tag, as `arda-settle` records them for each settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "snake_case")]
pub enum SiteTag {
    Ford,
    #[serde(rename = "bridge_site", alias = "bridge")]
    Bridge,
    Confluence,
    #[serde(alias = "harbor")]
    Harbour,
    Estuary,
    Pass,
    Defensible,
    Ore,
    Timber,
    Fish,
    Salt,
    Spring,
    River,
    Lake,
    Coast,
    Hill,
    Forest,
    Marsh,
    Mountain,
    Navigable,
    Arable,
}

impl SiteTag {
    /// Parses an `arda-settle` tag name (`"ford"`, `"bridge_site"`, ...).
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        use SiteTag as T;
        Some(match key.trim().to_ascii_lowercase().as_str() {
            "ford" => T::Ford,
            "bridge" | "bridge_site" => T::Bridge,
            "confluence" => T::Confluence,
            "harbour" | "harbor" => T::Harbour,
            "estuary" | "mouth" => T::Estuary,
            "pass" => T::Pass,
            "defensible" => T::Defensible,
            "ore" => T::Ore,
            "timber" => T::Timber,
            "fish" => T::Fish,
            "salt" => T::Salt,
            "spring" => T::Spring,
            "river" => T::River,
            "lake" => T::Lake,
            "coast" => T::Coast,
            "hill" => T::Hill,
            "forest" => T::Forest,
            "marsh" => T::Marsh,
            "mountain" => T::Mountain,
            "navigable" => T::Navigable,
            "arable" => T::Arable,
            _ => return None,
        })
    }

    /// Parses many keys, skipping unknown ones.
    pub fn from_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Vec<Self> {
        keys.into_iter().filter_map(Self::from_key).collect()
    }
}

/// What to name. `key` is the caller's stable identity for the place (a
/// settlement id, a river id): the same language, spec and key always give
/// the same name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceSpec {
    /// Place kind.
    pub kind: PlaceKind,
    /// Descriptive features the name should mention (`Oak`, `White`), in
    /// order of preference; empty lets the site and kind choose.
    pub features: Vec<Meaning>,
    /// Site tags of the place.
    pub site_tags: Vec<SiteTag>,
    /// Stable identity.
    #[serde(with = "crate::ids")]
    pub key: u64,
    /// For regions and realms: the capital (or region) to derive from.
    pub from: Option<Name>,
}

impl PlaceSpec {
    /// A spec with no features or tags.
    #[must_use]
    pub fn new(kind: PlaceKind, key: u64) -> Self {
        Self {
            kind,
            key,
            ..Self::default()
        }
    }

    /// Adds site tags.
    #[must_use]
    pub fn tags(mut self, tags: &[SiteTag]) -> Self {
        self.site_tags.extend_from_slice(tags);
        self
    }

    /// Adds features.
    #[must_use]
    pub fn features(mut self, features: &[Meaning]) -> Self {
        self.features.extend_from_slice(features);
        self
    }

    /// Derives from another name (regions, realms).
    #[must_use]
    pub fn from_name(mut self, name: &Name) -> Self {
        self.from = Some(name.clone());
        self
    }
}

/// One morpheme of a name, kept so names can be derived from names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// What it means (`None` for a meaningless personal-name root).
    pub meaning: Option<Meaning>,
    /// English form for glosses.
    pub gloss: String,
    /// Underlying phonemes, before dialect sound changes.
    pub ph: Vec<Ph>,
}

/// A generated name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name {
    /// The name in its own language ("Dervanford" becomes e.g. "Tarnagel").
    pub native: String,
    /// English rendering in English place-name style ("Oakford").
    pub gloss: String,
    /// Plain literal meaning ("ford of the oaks").
    pub literal: String,
    /// Respelling with the stressed syllable in capitals ("TAR-na-gel").
    pub pronunciation: String,
    /// Underlying phonemes of the whole name, before dialect changes.
    pub word: Vec<Ph>,
    /// Its morphemes, first to last.
    pub parts: Vec<Part>,
}
