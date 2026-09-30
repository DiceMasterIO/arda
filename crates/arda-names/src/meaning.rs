//! Meanings the lexicon carries a morpheme for, with their English glosses.
//!
//! Rule: every name element means something, so each native name has an
//! English gloss built the way English builds place names ("Oakford",
//! "Whitehill") and a literal reading ("ford of the oaks").

use serde::{Deserialize, Serialize};

/// What a meaning is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// A place-type head ("ford", "town").
    Head,
    /// A descriptive adjective ("white").
    Adj,
    /// A thing that can name a place ("oak", "stone").
    Noun,
    /// A person or being ("king"); glossed with a possessive.
    Being,
    /// A grammatical affix (plural, son of ...).
    Affix,
    /// A trade, for occupational family names.
    Trade,
}

/// Glosses and class of one meaning.
#[derive(Debug, Clone, Copy)]
pub struct MeaningInfo {
    /// Stable key, as used in data files.
    pub key: &'static str,
    /// Plain English word for literal readings.
    pub word: &'static str,
    /// English form as the first element of a compound ("Oak").
    pub modifier: &'static str,
    /// English form as the last element of a compound ("ford"), if a head.
    pub head: &'static str,
    /// English plural, empty for uncountable meanings.
    pub plural: &'static str,
    /// Use class.
    pub class: Class,
}

macro_rules! meanings {
    ($( $v:ident = $key:literal, $word:literal, $modi:literal, $head:literal, $pl:literal, $class:ident; )*) => {
        /// A meaning with a morpheme in every language.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub enum Meaning {
            $( #[doc = $word] #[serde(rename = $key)] $v, )*
        }

        /// Every meaning, in lexicon order.
        pub const ALL: &[Meaning] = &[ $( Meaning::$v, )* ];

        const TABLE: &[MeaningInfo] = &[ $( MeaningInfo {
            key: $key, word: $word, modifier: $modi, head: $head, plural: $pl, class: Class::$class,
        }, )* ];
    };
}

meanings! {
    // Place heads. The head gloss follows English toponymy.
    River = "river", "river", "River", "water", "rivers", Head;
    Brook = "brook", "brook", "Brook", "brook", "brooks", Head;
    Lake = "lake", "lake", "Mere", "mere", "lakes", Head;
    Sea = "sea", "sea", "Sea", "sea", "seas", Head;
    Bay = "bay", "bay", "Bay", "bay", "bays", Head;
    Harbour = "harbour", "harbour", "Haven", "haven", "harbours", Head;
    Mouth = "mouth", "river mouth", "Mouth", "mouth", "mouths", Head;
    Ford = "ford", "ford", "Ford", "ford", "fords", Head;
    Bridge = "bridge", "bridge", "Bridge", "bridge", "bridges", Head;
    Meet = "meet", "watersmeet", "Meet", "meet", "meetings", Head;
    Spring = "spring", "spring", "Well", "well", "springs", Head;
    Marsh = "marsh", "marsh", "Fen", "fen", "marshes", Head;
    Hill = "hill", "hill", "Hill", "hill", "hills", Head;
    Mount = "mount", "mountain", "Peak", "peak", "mountains", Head;
    Crag = "crag", "crag", "Crag", "crag", "crags", Head;
    Vale = "vale", "valley", "Dale", "dale", "valleys", Head;
    Wood = "wood", "forest", "Wood", "wood", "woods", Head;
    Grove = "grove", "grove", "Hurst", "hurst", "groves", Head;
    Field = "field", "field", "Field", "ley", "fields", Head;
    Island = "island", "island", "Holm", "holm", "islands", Head;
    Cape = "cape", "headland", "Ness", "ness", "headlands", Head;
    Strand = "strand", "shore", "Strand", "strand", "shores", Head;
    Pass = "pass", "pass", "Gate", "gate", "passes", Head;
    Town = "town", "town", "Town", "ton", "towns", Head;
    Village = "village", "village", "Wick", "wick", "villages", Head;
    Farm = "farm", "farmstead", "Stead", "stead", "farmsteads", Head;
    Fort = "fort", "fort", "Bury", "bury", "forts", Head;
    Hall = "hall", "hall", "Hall", "hall", "halls", Head;
    Shrine = "shrine", "shrine", "Kirk", "kirk", "shrines", Head;
    Market = "market", "market", "Market", "market", "markets", Head;
    Mine = "mine", "mine", "Delve", "delve", "mines", Head;
    Saltworks = "saltworks", "salt-works", "Wich", "wich", "salt-works", Head;
    Mill = "mill", "mill", "Mill", "mill", "mills", Head;
    Land = "land", "land", "Land", "land", "lands", Head;
    Shire = "shire", "shire", "Shire", "shire", "shires", Head;
    Realm = "realm", "realm", "Mark", "mark", "realms", Head;
    Clanhold = "clanhold", "hold", "Hold", "hold", "holds", Head;
    // Adjectives.
    Holy = "holy", "holy", "Holy", "", "", Adj;
    New = "new", "new", "New", "", "", Adj;
    Old = "old", "old", "Old", "", "", Adj;
    White = "white", "white", "White", "", "", Adj;
    Black = "black", "black", "Black", "", "", Adj;
    Red = "red", "red", "Red", "", "", Adj;
    Green = "green", "green", "Green", "", "", Adj;
    Grey = "grey", "grey", "Grey", "", "", Adj;
    Golden = "golden", "golden", "Gold", "", "", Adj;
    High = "high", "high", "High", "", "", Adj;
    Long = "long", "long", "Long", "", "", Adj;
    Broad = "broad", "broad", "Broad", "", "", Adj;
    Deep = "deep", "deep", "Deep", "", "", Adj;
    Cold = "cold", "cold", "Cold", "", "", Adj;
    Bright = "bright", "bright", "Bright", "", "", Adj;
    Dark = "dark", "dark", "Dark", "", "", Adj;
    Swift = "swift", "swift", "Swift", "", "", Adj;
    Still = "still", "still", "Still", "", "", Adj;
    Fair = "fair", "fair", "Fair", "", "", Adj;
    Little = "little", "little", "Little", "", "", Adj;
    Great = "great", "great", "Great", "", "", Adj;
    Upper = "upper", "upper", "Upper", "", "", Adj;
    Lower = "lower", "lower", "Nether", "", "", Adj;
    North = "north", "north", "North", "", "", Adj;
    South = "south", "south", "South", "", "", Adj;
    East = "east", "east", "East", "", "", Adj;
    West = "west", "west", "West", "", "", Adj;
    Wild = "wild", "wild", "Wild", "", "", Adj;
    Far = "far", "far", "Far", "", "", Adj;
    Clear = "clear", "clear", "Clear", "", "", Adj;
    Tall = "tall", "tall", "Tall", "", "", Adj;
    Young = "young", "young", "Young", "", "", Adj;
    Quiet = "quiet", "quiet", "Quiet", "", "", Adj;
    Merry = "merry", "merry", "Merry", "", "", Adj;
    Grim = "grim", "grim", "Grim", "", "", Adj;
    Strong = "strong", "strong", "Strong", "", "", Adj;
    Wise = "wise", "wise", "Wise", "", "", Adj;
    Bold = "bold", "bold", "Bold", "", "", Adj;
    Hard = "hard", "hard", "Hard", "", "", Adj;
    Keen = "keen", "keen", "Keen", "", "", Adj;
    Lucky = "lucky", "lucky", "Lucky", "", "", Adj;
    Stout = "stout", "stout", "Stout", "", "", Adj;
    // Things that name places.
    Oak = "oak", "oak", "Oak", "", "oaks", Noun;
    Ash = "ash", "ash tree", "Ash", "", "ashes", Noun;
    Birch = "birch", "birch", "Birch", "", "birches", Noun;
    Willow = "willow", "willow", "Willow", "", "willows", Noun;
    Thorn = "thorn", "thorn", "Thorn", "", "thorns", Noun;
    Reed = "reed", "reed", "Reed", "", "reeds", Noun;
    Pine = "pine", "pine", "Pine", "", "pines", Noun;
    Yew = "yew", "yew", "Yew", "", "yews", Noun;
    Apple = "apple", "apple", "Apple", "", "apples", Noun;
    Stone = "stone", "stone", "Stone", "", "stones", Noun;
    Iron = "iron", "iron", "Iron", "", "", Noun;
    Salt = "salt", "salt", "Salt", "", "", Noun;
    Gold = "gold", "gold", "Gold", "", "", Noun;
    Silver = "silver", "silver", "Silver", "", "", Noun;
    Copper = "copper", "copper", "Copper", "", "", Noun;
    Wolf = "wolf", "wolf", "Wolf", "", "wolves", Noun;
    Bear = "bear", "bear", "Bear", "", "bears", Noun;
    Raven = "raven", "raven", "Raven", "", "ravens", Noun;
    Eagle = "eagle", "eagle", "Eagle", "", "eagles", Noun;
    Hart = "hart", "hart", "Hart", "", "harts", Noun;
    Boar = "boar", "boar", "Boar", "", "boars", Noun;
    Horse = "horse", "horse", "Horse", "", "horses", Noun;
    Swan = "swan", "swan", "Swan", "", "swans", Noun;
    Heron = "heron", "heron", "Heron", "", "herons", Noun;
    Gull = "gull", "gull", "Gull", "", "gulls", Noun;
    Hawk = "hawk", "hawk", "Hawk", "", "hawks", Noun;
    Sheep = "sheep", "sheep", "Sheep", "", "sheep", Noun;
    Goat = "goat", "goat", "Goat", "", "goats", Noun;
    Cattle = "cattle", "cattle", "Kine", "", "", Noun;
    Fish = "fish", "fish", "Fish", "", "fish", Noun;
    Bee = "bee", "bee", "Honey", "", "bees", Noun;
    Barley = "barley", "barley", "Barley", "", "", Noun;
    Wind = "wind", "wind", "Wind", "", "winds", Noun;
    Snow = "snow", "snow", "Snow", "", "", Noun;
    Sun = "sun", "sun", "Sun", "", "", Noun;
    Moon = "moon", "moon", "Moon", "", "", Noun;
    Star = "star", "star", "Star", "", "stars", Noun;
    Fire = "fire", "fire", "Fire", "", "fires", Noun;
    Bell = "bell", "bell", "Bell", "", "bells", Noun;
    Horn = "horn", "horn", "Horn", "", "horns", Noun;
    Tower = "tower", "tower", "Tower", "", "towers", Noun;
    Wall = "wall", "wall", "Wall", "", "walls", Noun;
    Cross = "cross", "cross", "Cross", "", "crosses", Noun;
    Friend = "friend", "friend", "Friend", "", "friends", Noun;
    Peace = "peace", "peace", "Peace", "", "", Noun;
    War = "war", "war", "War", "", "wars", Noun;
    Glory = "glory", "glory", "Glory", "", "", Noun;
    Hope = "hope", "hope", "Hope", "", "", Noun;
    Heart = "heart", "heart", "Heart", "", "hearts", Noun;
    Hand = "hand", "hand", "Hand", "", "hands", Noun;
    Shield = "shield", "shield", "Shield", "", "shields", Noun;
    Spear = "spear", "spear", "Spear", "", "spears", Noun;
    Hammer = "hammer", "hammer", "Hammer", "", "hammers", Noun;
    Song = "song", "song", "Song", "", "songs", Noun;
    Dawn = "dawn", "dawn", "Dawn", "", "", Noun;
    // Beings, glossed with a possessive ("the king's town").
    King = "king", "king", "King", "", "kings", Being;
    Queen = "queen", "queen", "Queen", "", "queens", Being;
    Lord = "lord", "lord", "Lord", "", "lords", Being;
    Monk = "monk", "monk", "Monk", "", "monks", Being;
    Giant = "giant", "giant", "Giant", "", "giants", Being;
    Dragon = "dragon", "dragon", "Dragon", "", "dragons", Being;
    Guard = "guard", "guard", "Ward", "", "guards", Being;
    // Grammatical affixes.
    Plural = "plural", "(plural)", "", "", "", Affix;
    Of = "of", "of", "", "", "", Affix;
    Son = "son", "son of", "", "", "", Affix;
    Daughter = "daughter", "daughter of", "", "", "", Affix;
    Kin = "kin", "kin of", "", "", "", Affix;
    Feminine = "feminine", "(feminine)", "", "", "", Affix;
    Masculine = "masculine", "(masculine)", "", "", "", Affix;
    // Trades.
    Smith = "smith", "smith", "Smith", "", "smiths", Trade;
    Miller = "miller", "miller", "Miller", "", "millers", Trade;
    Baker = "baker", "baker", "Baker", "", "bakers", Trade;
    Brewer = "brewer", "brewer", "Brewer", "", "brewers", Trade;
    Weaver = "weaver", "weaver", "Weaver", "", "weavers", Trade;
    Tailor = "tailor", "tailor", "Taylor", "", "tailors", Trade;
    Tanner = "tanner", "tanner", "Tanner", "", "tanners", Trade;
    Cooper = "cooper", "cooper", "Cooper", "", "coopers", Trade;
    Wright = "wright", "carpenter", "Wright", "", "carpenters", Trade;
    Mason = "mason", "mason", "Mason", "", "masons", Trade;
    Potter = "potter", "potter", "Potter", "", "potters", Trade;
    Fisher = "fisher", "fisher", "Fisher", "", "fishers", Trade;
    Hunter = "hunter", "hunter", "Hunter", "", "hunters", Trade;
    Shepherd = "shepherd", "shepherd", "Shepherd", "", "shepherds", Trade;
    Merchant = "merchant", "merchant", "Chapman", "", "merchants", Trade;
    Priest = "priest", "priest", "Priest", "", "priests", Trade;
    Taverner = "taverner", "innkeeper", "Taverner", "", "innkeepers", Trade;
    Miner = "miner", "miner", "Miner", "", "miners", Trade;
    Forester = "forester", "forester", "Forester", "", "foresters", Trade;
    Sailor = "sailor", "sailor", "Shipman", "", "sailors", Trade;
    Healer = "healer", "healer", "Leech", "", "healers", Trade;
    Scribe = "scribe", "scribe", "Clerk", "", "scribes", Trade;
    Cook = "cook", "cook", "Cook", "", "cooks", Trade;
    Butcher = "butcher", "butcher", "Butcher", "", "butchers", Trade;
    Carter = "carter", "carter", "Carter", "", "carters", Trade;
    Thatcher = "thatcher", "thatcher", "Thatcher", "", "thatchers", Trade;
    Farmer = "farmer", "farmer", "Tiller", "", "farmers", Trade;
    Soldier = "soldier", "soldier", "Sergeant", "", "soldiers", Trade;
    Herder = "herder", "herder", "Herd", "", "herders", Trade;
}

impl Meaning {
    /// Glosses and class.
    #[must_use]
    pub fn info(self) -> &'static MeaningInfo {
        // The macro builds ALL and TABLE from one list, so the index exists.
        TABLE.get(self as usize).unwrap_or(&TABLE[0])
    }

    /// Looks a meaning up by key, with a few trade synonyms
    /// ("blacksmith" → smith).
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        let k = key.trim().to_ascii_lowercase();
        let k = match k.as_str() {
            "blacksmith" | "armourer" | "armorer" | "farrier" => "smith",
            "carpenter" | "joiner" | "wheelwright" => "wright",
            "innkeeper" | "barkeep" | "tavernkeeper" | "publican" => "taverner",
            "fisherman" | "fishwife" => "fisher",
            "trader" | "shopkeeper" | "peddler" | "pedlar" => "merchant",
            "cleric" | "acolyte" => "priest",
            "physician" | "herbalist" | "midwife" => "healer",
            "clerk" | "sage" => "scribe",
            "woodcutter" | "lumberjack" | "ranger" => "forester",
            "seaman" | "boatman" | "mariner" => "sailor",
            "stonemason" => "mason",
            "watchman" | "militia" | "guardsman" => "soldier",
            "labourer" | "laborer" | "peasant" => "farmer",
            "herdsman" | "cowherd" | "swineherd" | "goatherd" => "herder",
            other => other,
        };
        ALL.iter().copied().find(|m| m.info().key == k)
    }

    /// Every meaning of one class.
    pub fn of_class(class: Class) -> impl Iterator<Item = Meaning> {
        ALL.iter().copied().filter(move |m| m.info().class == class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn table_is_consistent() {
        assert_eq!(ALL.len(), TABLE.len());
        let keys: BTreeSet<_> = ALL.iter().map(|m| m.info().key).collect();
        assert_eq!(keys.len(), ALL.len());
        for m in ALL {
            assert_eq!(Meaning::from_key(m.info().key), Some(*m));
            if m.info().class == Class::Head {
                assert!(!m.info().head.is_empty());
            }
        }
        assert_eq!(Meaning::from_key("Blacksmith"), Some(Meaning::Smith));
    }
}
