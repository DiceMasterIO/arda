//! Building functions and workshop crafts (`vocabulary.md` "Building
//! functions" and I6/I7; `logic/10` §town-function-keys).
//!
//! The serde form is a plain snake_case string, never a tagged object. A
//! workshop's craft is not folded into the function: it travels beside it,
//! as a `craft` field or a `craft:<key>` tag ([`Craft::tag`]).

use serde::{Deserialize, Serialize};

/// What a building is for: the vocabulary list plus `market_hall`, `mine`,
/// `lumber_camp` and `school`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum BuildingFunction {
    /// A town house.
    House,
    /// A farmstead's dwelling.
    Farmhouse,
    /// A small cottage.
    Cottage,
    /// A noble manor.
    Manor,
    /// An inn with rooms.
    Inn,
    /// A drinking house.
    Tavern,
    /// A bakery.
    Bakery,
    /// A brewery.
    Brewery,
    /// A water or wind mill.
    Mill,
    /// A forge.
    Smithy,
    /// A workshop; its craft travels beside the function (see [`Craft`]).
    Workshop,
    /// A tannery.
    Tannery,
    /// An apothecary's shop.
    Apothecary,
    /// A temple.
    Temple,
    /// A wayside or village shrine.
    Shrine,
    /// A library or archive.
    Library,
    /// A merchant's warehouse.
    Warehouse,
    /// An open market place.
    Market,
    /// A market stall or shopfront.
    Stall,
    /// A quay or dock.
    Dock,
    /// A boathouse or slipway.
    Boathouse,
    /// A lord's keep or castle.
    Keep,
    /// Garrison barracks.
    Barracks,
    /// A watch house.
    Guardhouse,
    /// Stables.
    Stable,
    /// A barn.
    Barn,
    /// A street (outdoor area).
    Street,
    /// A farm yard (outdoor area).
    Farm,
    /// A covered market hall.
    MarketHall,
    /// A mine or quarry head.
    Mine,
    /// A lumber camp.
    LumberCamp,
    /// A school.
    School,
}

impl BuildingFunction {
    /// Every function, vocabulary order first, then the four additions.
    pub const ALL: [Self; 32] = [
        Self::House,
        Self::Farmhouse,
        Self::Cottage,
        Self::Manor,
        Self::Inn,
        Self::Tavern,
        Self::Bakery,
        Self::Brewery,
        Self::Mill,
        Self::Smithy,
        Self::Workshop,
        Self::Tannery,
        Self::Apothecary,
        Self::Temple,
        Self::Shrine,
        Self::Library,
        Self::Warehouse,
        Self::Market,
        Self::Stall,
        Self::Dock,
        Self::Boathouse,
        Self::Keep,
        Self::Barracks,
        Self::Guardhouse,
        Self::Stable,
        Self::Barn,
        Self::Street,
        Self::Farm,
        Self::MarketHall,
        Self::Mine,
        Self::LumberCamp,
        Self::School,
    ];

    /// The serde key, also the catalogue's `function:` tag value.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::House => "house",
            Self::Farmhouse => "farmhouse",
            Self::Cottage => "cottage",
            Self::Manor => "manor",
            Self::Inn => "inn",
            Self::Tavern => "tavern",
            Self::Bakery => "bakery",
            Self::Brewery => "brewery",
            Self::Mill => "mill",
            Self::Smithy => "smithy",
            Self::Workshop => "workshop",
            Self::Tannery => "tannery",
            Self::Apothecary => "apothecary",
            Self::Temple => "temple",
            Self::Shrine => "shrine",
            Self::Library => "library",
            Self::Warehouse => "warehouse",
            Self::Market => "market",
            Self::Stall => "stall",
            Self::Dock => "dock",
            Self::Boathouse => "boathouse",
            Self::Keep => "keep",
            Self::Barracks => "barracks",
            Self::Guardhouse => "guardhouse",
            Self::Stable => "stable",
            Self::Barn => "barn",
            Self::Street => "street",
            Self::Farm => "farm",
            Self::MarketHall => "market_hall",
            Self::Mine => "mine",
            Self::LumberCamp => "lumber_camp",
            Self::School => "school",
        }
    }

    /// Parses a serde key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// The namespaced catalogue tag, `function:<key>` (vocabulary I8).
    #[must_use]
    pub fn tag(self) -> String {
        format!("function:{}", self.key())
    }

    /// Whether the building is first of all a home (a manor is a seat, not a
    /// plain dwelling, as in `arda-npc`).
    #[must_use]
    pub const fn is_dwelling(self) -> bool {
        matches!(self, Self::House | Self::Farmhouse | Self::Cottage)
    }
}

/// The trade practised in a [`BuildingFunction::Workshop`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Craft {
    /// Carpenter.
    Carpentry,
    /// Weaver.
    Weaving,
    /// Potter.
    Pottery,
    /// Cobbler.
    Cobbling,
    /// Mason.
    Masonry,
    /// Jeweller.
    Jewellery,
    /// Glassblower.
    Glassblowing,
    /// Leatherworker.
    Leatherwork,
    /// Woodcarver.
    Woodcarving,
    /// Tinker.
    Tinkering,
    /// Painter.
    Painting,
    /// Cartographer.
    Cartography,
}

impl Craft {
    /// Every craft.
    pub const ALL: [Self; 12] = [
        Self::Carpentry,
        Self::Weaving,
        Self::Pottery,
        Self::Cobbling,
        Self::Masonry,
        Self::Jewellery,
        Self::Glassblowing,
        Self::Leatherwork,
        Self::Woodcarving,
        Self::Tinkering,
        Self::Painting,
        Self::Cartography,
    ];

    /// The serde key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Carpentry => "carpentry",
            Self::Weaving => "weaving",
            Self::Pottery => "pottery",
            Self::Cobbling => "cobbling",
            Self::Masonry => "masonry",
            Self::Jewellery => "jewellery",
            Self::Glassblowing => "glassblowing",
            Self::Leatherwork => "leatherwork",
            Self::Woodcarving => "woodcarving",
            Self::Tinkering => "tinkering",
            Self::Painting => "painting",
            Self::Cartography => "cartography",
        }
    }

    /// The free catalogue tag, `craft:<key>`.
    #[must_use]
    pub fn tag(self) -> String {
        format!("craft:{}", self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_plain_strings_and_round_trip() {
        for f in BuildingFunction::ALL {
            let json = serde_json::to_string(&f).unwrap();
            assert_eq!(json, format!("\"{}\"", f.key()));
            assert_eq!(serde_json::from_str::<BuildingFunction>(&json).unwrap(), f);
            assert_eq!(BuildingFunction::from_key(f.key()), Some(f));
        }
        for c in Craft::ALL {
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.key())
            );
        }
        assert_eq!(BuildingFunction::MarketHall.tag(), "function:market_hall");
        assert_eq!(Craft::Weaving.tag(), "craft:weaving");
        assert_eq!(BuildingFunction::from_key("castle"), None);
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<&str> = BuildingFunction::ALL.iter().map(|f| f.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), BuildingFunction::ALL.len());
    }
}
