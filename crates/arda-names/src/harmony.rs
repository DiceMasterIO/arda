//! Vowel harmony: front and back vowel classes, with /i/ and schwa neutral.
//!
//! Rule: within a root all non-neutral vowels share a class; an affix
//! takes the class of the last non-neutral vowel before it.

use crate::phoneme::{Manner, Ph};

/// Harmony class of a vowel: `Some(true)` front, `Some(false)` back,
/// `None` for neutral vowels and consonants.
#[must_use]
pub fn front_class(p: Ph) -> Option<bool> {
    let i = p.info();
    if i.manner != Manner::Vowel || i.code.starts_with('i') || i.code == "@" {
        return None;
    }
    Some(p.is_front())
}

/// All non-neutral vowels agree in frontness.
#[must_use]
pub fn harmonic(w: &[Ph]) -> bool {
    let mut seen = None;
    for &p in w {
        if let Some(f) = front_class(p) {
            if seen.is_some_and(|s| s != f) {
                return false;
            }
            seen = Some(f);
        }
    }
    true
}

/// Frontness of the last non-neutral vowel.
#[must_use]
pub fn last_class(w: &[Ph]) -> Option<bool> {
    w.iter().rev().find_map(|&p| front_class(p))
}

/// Moves a vowel to the other harmony class (a↔e, o↔e, u↔ue).
#[must_use]
pub fn to_class(p: Ph, front: bool) -> Ph {
    if front_class(p).is_none_or(|f| f == front) {
        return p;
    }
    let code = match (p.info().code, front) {
        ("a" | "o", true) => "e",
        ("u", true) => "ue",
        ("a:" | "o:", true) => "e:",
        ("u:", true) => "i:",
        ("e", false) => "a",
        ("ue", false) => "u",
        ("e:", false) => "a:",
        (c, _) => c,
    };
    Ph::code(code)
}
