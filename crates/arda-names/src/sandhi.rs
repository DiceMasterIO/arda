//! Repairs a word that breaks its language's phonotactics, as happens where
//! morphemes meet in a compound or a foreign word is borrowed.
//!
//! Rules, applied to the first fault until none is left:
//! - vowels that may not meet: the first is elided (identical vowels merge);
//! - a bad initial cluster loses its first consonant;
//! - a bad final cluster loses its last consonant, and a sound that may not
//!   end a word turns into the nearest coda or takes an echo vowel;
//! - a bad medial run loses the one consonant that makes it valid, or else
//!   an epenthetic vowel breaks it.

use crate::phoneme::Ph;
use crate::phonology::{Fault, Phonology};

/// How close (feature distance) a replacement coda must be.
const NEAR_CODA: u32 = 3;

/// Repairs `w` in place. Every step shortens a consonant run or removes a
/// vowel, so the loop is bounded; the cap only guards against surprises.
pub fn repair(ph: &Phonology, w: &mut Vec<Ph>) {
    adapt(ph, w);
    for _ in 0..32 {
        let Some((s, e, fault)) = ph.first_violation(w) else {
            return;
        };
        match fault {
            Fault::NoVowel => w.push(ph.epenthetic),
            Fault::Vowels => {
                if e - s >= 3 {
                    w.remove(s + 1);
                } else {
                    w.remove(s);
                }
            }
            Fault::Initial => {
                if e - s >= 2 {
                    w.remove(s);
                } else {
                    w.remove(s);
                    if w.first().is_some_and(|p| p.is_vowel()) && ph.initial_onset > 950 {
                        let c = ph.nearest_consonant(Ph::code("h"), false);
                        w.insert(0, c);
                    }
                }
            }
            Fault::Final => fix_final(ph, w, s, e),
            Fault::Medial => fix_medial(ph, w, s, e),
        }
    }
}

/// Replaces sounds outside the inventory with their nearest neighbours.
pub fn adapt(ph: &Phonology, w: &mut [Ph]) {
    for p in w.iter_mut() {
        if p.is_vowel() {
            if !ph.vowels.iter().any(|v| v.0 == *p) {
                *p = ph.nearest_vowel(*p);
            }
        } else if !ph.has_consonant(*p) {
            *p = ph.nearest_consonant(*p, false);
        }
    }
}

fn fix_final(ph: &Phonology, w: &mut Vec<Ph>, s: usize, e: usize) {
    if e - s >= 2 {
        // Keep whichever consonant is a legal coda by itself.
        if ph.is_coda(&w[s..s + 1]) || !ph.is_coda(&w[e - 1..e]) {
            w.truncate(e - 1);
        } else {
            w.remove(s);
        }
        return;
    }
    let p = w[s];
    let near = ph.nearest_consonant(p, true);
    if ph.is_coda(&[near]) && p.distance(near) <= NEAR_CODA {
        w[s] = near;
    } else {
        w.push(ph.epenthetic);
    }
}

fn fix_medial(ph: &Phonology, w: &mut Vec<Ph>, s: usize, e: usize) {
    let run: Vec<Ph> = w[s..e].to_vec();
    if let Some(i) = run.windows(2).position(|p| p[0] == p[1]) {
        w.remove(s + i);
        return;
    }
    if let Some(i) = run.iter().skip(1).position(|&p| p == Ph::code("h")) {
        w.remove(s + i + 1);
        return;
    }
    if run.len() <= ph.max_medial + 1 {
        for i in 0..run.len() {
            let mut shorter = run.clone();
            shorter.remove(i);
            if !shorter.is_empty() && ph.medial_ok(&shorter) {
                w.remove(s + i);
                return;
            }
        }
    }
    // Break the run after its longest legal coda.
    let k = (1..run.len())
        .rev()
        .find(|&k| ph.is_coda(&run[..k]))
        .unwrap_or(1);
    w.insert(s + k, ph.epenthetic);
}
