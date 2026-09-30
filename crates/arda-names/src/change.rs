//! Regular sound changes, the stuff dialects are made of.
//!
//! Rule: a sound change rewrites every matching phoneme in its context
//! (Neogrammarian regularity), so all words of a dialect shift together and
//! neighbouring dialects that share a change keep sounding related.

use crate::phoneme::{Manner, Ph};
use serde::{Deserialize, Serialize};

/// One regular sound change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundChange {
    /// Voiceless stops voice between vowels (t → d).
    Lenition,
    /// Voiced stops become fricatives between vowels (d → dh).
    Spirantisation,
    /// Voiced obstruents devoice at the end of a word (d → t).
    FinalDevoicing,
    /// /k g/ become /ch j/ before front vowels.
    Palatalisation,
    /// One sound becomes another everywhere (vowel shifts, mergers).
    Shift {
        /// Old sound.
        from: Ph,
        /// New sound.
        to: Ph,
    },
    /// A final vowel is lost in words of two or more syllables.
    Apocope,
    /// /h/ is lost.
    HDropping,
    /// /s z/ become /r/ between vowels.
    Rhotacism,
    /// /s/ becomes /sh/ before a consonant.
    SibilantShift,
    /// Diphthongs become long vowels (ai → e:).
    Monophthongisation,
    /// Word-initial fricatives voice (f → v).
    InitialVoicing,
    /// A word-final /n/ after a vowel is lost.
    FinalNasalLoss,
    /// Long vowels become diphthongs (i: → ai).
    Diphthongisation,
}

fn swap(p: Ph, pairs: &[(&str, &str)]) -> Ph {
    let code = p.info().code;
    pairs
        .iter()
        .find(|(a, _)| *a == code)
        .map_or(p, |(_, b)| Ph::code(b))
}

const VOICE: [(&str, &str); 9] = [
    ("p", "b"),
    ("t", "d"),
    ("k", "g"),
    ("f", "v"),
    ("th", "dh"),
    ("s", "z"),
    ("sh", "zh"),
    ("x", "gh"),
    ("ch", "dj"),
];

fn voiced(p: Ph) -> Ph {
    swap(p, &VOICE)
}

fn devoiced(p: Ph) -> Ph {
    let rev: Vec<(&str, &str)> = VOICE.iter().map(|&(a, b)| (b, a)).collect();
    swap(p, &rev)
}

fn between_vowels(w: &[Ph], i: usize) -> bool {
    i > 0 && w[i - 1].is_vowel() && w.get(i + 1).is_some_and(|p| p.is_vowel())
}

impl SoundChange {
    /// Every context-driven change (shifts are generated per inventory).
    pub const GENERAL: [SoundChange; 12] = [
        SoundChange::Lenition,
        SoundChange::Spirantisation,
        SoundChange::FinalDevoicing,
        SoundChange::Palatalisation,
        SoundChange::Apocope,
        SoundChange::HDropping,
        SoundChange::Rhotacism,
        SoundChange::SibilantShift,
        SoundChange::Monophthongisation,
        SoundChange::InitialVoicing,
        SoundChange::FinalNasalLoss,
        SoundChange::Diphthongisation,
    ];

    /// Applies the change to a word.
    #[must_use]
    pub fn apply(self, w: &[Ph]) -> Vec<Ph> {
        let mut out = Vec::with_capacity(w.len() + 1);
        let n = w.len();
        let vowels = w.iter().filter(|p| p.is_vowel()).count();
        for (i, &p) in w.iter().enumerate() {
            let info = p.info();
            match self {
                SoundChange::Lenition if info.manner == Manner::Stop && between_vowels(w, i) => {
                    out.push(voiced(p));
                }
                SoundChange::Spirantisation
                    if info.manner == Manner::Stop && info.voiced && between_vowels(w, i) =>
                {
                    out.push(swap(p, &[("b", "v"), ("d", "dh"), ("g", "gh")]));
                }
                SoundChange::FinalDevoicing if i + 1 == n && p.is_obstruent() => {
                    out.push(devoiced(p));
                }
                SoundChange::Palatalisation if w.get(i + 1).is_some_and(|q| q.is_front()) => {
                    out.push(swap(p, &[("k", "ch"), ("g", "dj")]));
                }
                SoundChange::Shift { from, to } if p == from => out.push(to),
                SoundChange::Apocope if i + 1 == n && p.is_vowel() && vowels >= 2 && n >= 3 => {
                    if w[i - 1].is_vowel() {
                        out.push(p);
                    }
                }
                SoundChange::HDropping if info.code == "h" => {}
                SoundChange::Rhotacism
                    if matches!(info.code, "s" | "z") && between_vowels(w, i) =>
                {
                    out.push(Ph::code("r"));
                }
                SoundChange::SibilantShift
                    if info.code == "s" && w.get(i + 1).is_some_and(|q| q.is_consonant()) =>
                {
                    out.push(Ph::code("sh"));
                }
                SoundChange::Monophthongisation
                    if p.is_vowel() && i > 0 && w[i - 1].is_vowel() && !w[i - 1].info().long =>
                {
                    let merged = match (w[i - 1].info().code, info.code) {
                        ("a", "i" | "e") => "e:",
                        ("a" | "o", "u") => "o:",
                        ("e", "i") => "i:",
                        _ => "",
                    };
                    if merged.is_empty() {
                        out.push(p);
                    } else if let Some(last) = out.last_mut() {
                        *last = Ph::code(merged);
                    }
                }
                SoundChange::InitialVoicing if i == 0 && info.manner == Manner::Fricative => {
                    out.push(voiced(p));
                }
                SoundChange::FinalNasalLoss if i + 1 == n && info.code == "n" && vowels >= 2 => {}
                SoundChange::Diphthongisation if info.long => {
                    let (a, b) = match info.code {
                        "i:" => ("a", "i"),
                        "u:" => ("a", "u"),
                        "e:" => ("e", "i"),
                        "o:" => ("o", "u"),
                        _ => ("a", "e"),
                    };
                    out.push(Ph::code(a));
                    out.push(Ph::code(b));
                }
                _ => out.push(p),
            }
        }
        out
    }

    /// Plausible unconditional shifts for vowels and a few consonants.
    #[must_use]
    pub fn shifts() -> Vec<SoundChange> {
        const PAIRS: [(&str, &str); 14] = [
            ("a", "o"),
            ("a", "e"),
            ("o", "u"),
            ("e", "i"),
            ("u", "o"),
            ("i", "e"),
            ("u", "ue"),
            ("a:", "o:"),
            ("e:", "i:"),
            ("th", "t"),
            ("x", "h"),
            ("w", "v"),
            ("v", "w"),
            ("k", "ch"),
        ];
        PAIRS
            .iter()
            .map(|&(a, b)| SoundChange::Shift {
                from: Ph::code(a),
                to: Ph::code(b),
            })
            .collect()
    }
}

/// Keeps a changed word pronounceable without forcing it back into the
/// parent inventory: at most three consonants or two vowels in a row, no
/// doubled consonant at a word edge, and at least one vowel.
pub(crate) fn tidy(w: &mut Vec<Ph>, epenthetic: Ph) {
    let mut i = 0;
    let (mut cons, mut vows) = (0, 0);
    while i < w.len() {
        if w[i].is_vowel() {
            vows += 1;
            cons = 0;
            if vows > 2 || (i > 0 && w[i - 1] == w[i]) {
                w.remove(i);
                vows -= 1;
                continue;
            }
        } else {
            cons += 1;
            vows = 0;
            let edge = i == 1 || i + 1 == w.len();
            if cons > 3 || (i > 0 && w[i - 1] == w[i] && edge) {
                if cons > 3 {
                    w.insert(i, epenthetic);
                    cons = 0;
                } else {
                    w.remove(i);
                    cons -= 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    if !w.iter().any(|p| p.is_vowel()) {
        w.push(epenthetic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(codes: &str) -> Vec<Ph> {
        codes.split(' ').map(Ph::code).collect()
    }

    #[test]
    fn changes_apply_in_context() {
        assert_eq!(SoundChange::Lenition.apply(&w("a t a t")), w("a d a t"));
        assert_eq!(SoundChange::FinalDevoicing.apply(&w("b a d")), w("b a t"));
        assert_eq!(
            SoundChange::Palatalisation.apply(&w("k e k a")),
            w("ch e k a")
        );
        assert_eq!(SoundChange::Apocope.apply(&w("t a l a")), w("t a l"));
        assert_eq!(
            SoundChange::Monophthongisation.apply(&w("t a i n")),
            w("t e: n")
        );
        assert_eq!(SoundChange::Rhotacism.apply(&w("a s a")), w("a r a"));
    }

    #[test]
    fn tidy_breaks_long_runs() {
        let mut x = w("a r s t k a");
        tidy(&mut x, Ph::code("e"));
        assert_eq!(x, w("a r s t e k a"));
    }
}
