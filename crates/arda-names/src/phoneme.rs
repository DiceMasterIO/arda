//! The universal phoneme table every language draws its inventory from.
//!
//! Rule: phonemes carry articulatory features, so sound changes and loanword
//! adaptation work on features ("voiceless stop between vowels") rather than
//! on spellings.

use serde::{Deserialize, Serialize};

/// A phoneme: an index into [`TABLE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Ph(pub u8);

/// Manner of articulation (vowels have their own class).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manner {
    /// Plosive.
    Stop,
    /// Fricative.
    Fricative,
    /// Affricate.
    Affricate,
    /// Nasal.
    Nasal,
    /// Lateral or rhotic.
    Liquid,
    /// Semivowel.
    Glide,
    /// Vowel.
    Vowel,
}

/// Place of articulation, front of the mouth to back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    /// Lips.
    Labial,
    /// Teeth.
    Dental,
    /// Alveolar ridge.
    Alveolar,
    /// Behind the ridge.
    Postalveolar,
    /// Hard palate.
    Palatal,
    /// Soft palate.
    Velar,
    /// Uvula.
    Uvular,
    /// Glottis.
    Glottal,
}

/// Features of one phoneme.
#[derive(Debug, Clone, Copy)]
pub struct Info {
    /// Short ASCII code used in preset tables.
    pub code: &'static str,
    /// Default romanisation.
    pub spell: &'static str,
    /// Manner class.
    pub manner: Manner,
    /// Place (for vowels: front = Palatal, central = Postalveolar, back = Velar).
    pub place: Place,
    /// Voiced.
    pub voiced: bool,
    /// Vowel height 0 (low) to 2 (high); 0 for consonants.
    pub height: u8,
    /// Rounded vowel.
    pub round: bool,
    /// Long vowel.
    pub long: bool,
}

const fn c(
    code: &'static str,
    spell: &'static str,
    manner: Manner,
    place: Place,
    voiced: bool,
) -> Info {
    Info {
        code,
        spell,
        manner,
        place,
        voiced,
        height: 0,
        round: false,
        long: false,
    }
}

const fn v(
    code: &'static str,
    spell: &'static str,
    place: Place,
    height: u8,
    round: bool,
    long: bool,
) -> Info {
    Info {
        code,
        spell,
        manner: Manner::Vowel,
        place,
        voiced: true,
        height,
        round,
        long,
    }
}

use Manner::{
    Affricate as Af, Fricative as Fr, Glide as Gl, Liquid as Li, Nasal as Na, Stop as St,
};
use Place::{
    Alveolar as Alv, Dental as Den, Glottal as Glo, Labial as Lab, Palatal as Pal,
    Postalveolar as Pos, Uvular as Uvu, Velar as Vel,
};

/// Every phoneme any language may use.
pub const TABLE: [Info; 42] = [
    c("p", "p", St, Lab, false),
    c("b", "b", St, Lab, true),
    c("t", "t", St, Alv, false),
    c("d", "d", St, Alv, true),
    c("k", "k", St, Vel, false),
    c("g", "g", St, Vel, true),
    c("q", "q", St, Uvu, false),
    c("'", "'", St, Glo, false),
    c("f", "f", Fr, Lab, false),
    c("v", "v", Fr, Lab, true),
    c("th", "th", Fr, Den, false),
    c("dh", "dh", Fr, Den, true),
    c("s", "s", Fr, Alv, false),
    c("z", "z", Fr, Alv, true),
    c("sh", "sh", Fr, Pos, false),
    c("zh", "zh", Fr, Pos, true),
    c("x", "kh", Fr, Vel, false),
    c("gh", "gh", Fr, Vel, true),
    c("h", "h", Fr, Glo, false),
    c("ch", "ch", Af, Pos, false),
    c("dj", "j", Af, Pos, true),
    c("ts", "ts", Af, Alv, false),
    c("m", "m", Na, Lab, true),
    c("n", "n", Na, Alv, true),
    c("ng", "ng", Na, Vel, true),
    c("ny", "ny", Na, Pal, true),
    c("l", "l", Li, Alv, true),
    c("r", "r", Li, Alv, true),
    c("w", "w", Gl, Lab, true),
    c("y", "y", Gl, Pal, true),
    v("a", "a", Pos, 0, false, false),
    v("e", "e", Pal, 1, false, false),
    v("i", "i", Pal, 2, false, false),
    v("o", "o", Vel, 1, true, false),
    v("u", "u", Vel, 2, true, false),
    v("ue", "y", Pal, 2, true, false),
    v("@", "e", Pos, 1, false, false),
    v("a:", "aa", Pos, 0, false, true),
    v("e:", "ee", Pal, 1, false, true),
    v("i:", "ii", Pal, 2, false, true),
    v("o:", "oo", Vel, 1, true, true),
    v("u:", "uu", Vel, 2, true, true),
];

impl Ph {
    /// Looks a phoneme up by its table code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        TABLE
            .iter()
            .position(|i| i.code == code)
            .and_then(|p| u8::try_from(p).ok())
            .map(Ph)
    }

    /// Looks a phoneme up by code, falling back to schwa for unknown codes
    /// (preset tables are checked by a unit test, so this never fires).
    #[must_use]
    pub fn code(code: &str) -> Self {
        Self::from_code(code).unwrap_or(Ph(36))
    }

    /// Its features.
    #[must_use]
    pub fn info(self) -> &'static Info {
        TABLE.get(usize::from(self.0)).unwrap_or(&TABLE[36])
    }

    /// Is a vowel.
    #[must_use]
    pub fn is_vowel(self) -> bool {
        self.info().manner == Manner::Vowel
    }

    /// Is a consonant (glides included).
    #[must_use]
    pub fn is_consonant(self) -> bool {
        !self.is_vowel()
    }

    /// Front vowel (triggers palatalisation).
    #[must_use]
    pub fn is_front(self) -> bool {
        let i = self.info();
        i.manner == Manner::Vowel && i.place == Place::Palatal
    }

    /// Obstruent: stop, fricative or affricate.
    #[must_use]
    pub fn is_obstruent(self) -> bool {
        matches!(
            self.info().manner,
            Manner::Stop | Manner::Fricative | Manner::Affricate
        )
    }

    /// Sonority rank for onset and coda ordering (higher is more vowel-like).
    #[must_use]
    pub fn sonority(self) -> u8 {
        match self.info().manner {
            Manner::Stop | Manner::Affricate => 1,
            Manner::Fricative => 2,
            Manner::Nasal => 3,
            Manner::Liquid => 4,
            Manner::Glide => 5,
            Manner::Vowel => 6,
        }
    }

    /// Feature distance used to adapt foreign sounds to an inventory.
    #[must_use]
    pub fn distance(self, other: Self) -> u32 {
        let (a, b) = (self.info(), other.info());
        if a.manner == Manner::Vowel || b.manner == Manner::Vowel {
            if a.manner != b.manner {
                return 100;
            }
            let place = (a.place as i32 - b.place as i32).unsigned_abs();
            let height = u32::from(a.height.abs_diff(b.height));
            return place * 2
                + height * 3
                + u32::from(a.round != b.round)
                + u32::from(a.long != b.long);
        }
        let manner = if a.manner == b.manner { 0 } else { 4 };
        let place = (a.place as i32 - b.place as i32).unsigned_abs() * 2;
        manner + place + u32::from(a.voiced != b.voiced)
    }

    /// The same vowel made short (identity for consonants and short vowels).
    #[must_use]
    pub fn shortened(self) -> Self {
        let code = self.info().code;
        match code.strip_suffix(':') {
            Some(short) => Self::code(short),
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_round_trip() {
        for (i, info) in TABLE.iter().enumerate() {
            let p = Ph::from_code(info.code).map(|p| usize::from(p.0));
            assert_eq!(p, Some(i), "{}", info.code);
        }
        assert_eq!(Ph::code("@").info().code, "@");
        assert_eq!(Ph::code("a:").shortened(), Ph::code("a"));
    }

    #[test]
    fn distance_prefers_near_sounds() {
        let (t, d, k, a) = (Ph::code("t"), Ph::code("d"), Ph::code("k"), Ph::code("a"));
        assert!(t.distance(d) < t.distance(k));
        assert!(t.distance(k) < t.distance(a));
    }
}
