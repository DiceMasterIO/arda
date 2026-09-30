//! Phoneme inventory and phonotactics of one language: what a syllable may
//! look like, how words are built, checked and repaired.
//!
//! Rules:
//! - An onset is one inventory consonant or a listed cluster; a coda is one
//!   listed coda or a listed coda cluster. A word-medial consonant run is
//!   valid when some split makes it coda + onset.
//! - Vowels meet only as listed diphthongs, or across syllables when the
//!   language allows hiatus.
//! - /h/ never follows a consonant; doubled consonants appear only in
//!   languages with geminates, and only for single-letter sounds.
//! - With vowel harmony, a root's vowels are all front or all back (/i/ is
//!   neutral) and affix vowels follow the stem.

use crate::harmony::{front_class, harmonic};
use crate::phoneme::Ph;
use crate::preset::{Spec, Stress};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Maximum consonants in a row anywhere in a word.
pub const MAX_RUN: usize = 3;

/// Inventory and phonotactic rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phonology {
    /// Consonants and weights.
    pub consonants: Vec<(Ph, u32)>,
    /// Vowels and weights.
    pub vowels: Vec<(Ph, u32)>,
    /// Allowed diphthongs.
    pub diphthongs: Vec<(Ph, Ph)>,
    /// Allowed onset clusters.
    pub clusters: Vec<Vec<Ph>>,
    /// Single codas and weights.
    pub codas: Vec<(Ph, u32)>,
    /// Allowed coda clusters.
    pub coda_clusters: Vec<Vec<Ph>>,
    /// Never word-initial.
    pub no_initial: Vec<Ph>,
    /// Per mille: word starts with a consonant.
    pub initial_onset: u32,
    /// Per mille: onset is a cluster.
    pub cluster_rate: u32,
    /// Per mille: non-final syllable is closed.
    pub medial_coda: u32,
    /// Per mille: word ends in a consonant.
    pub final_coda: u32,
    /// Per mille: nucleus is a diphthong.
    pub diph_rate: u32,
    /// Vowels may meet across syllables.
    pub hiatus: bool,
    /// Vowel harmony.
    pub harmony: bool,
    /// Geminates allowed.
    pub geminates: bool,
    /// Most consonants in a row inside a word.
    pub max_medial: usize,
    /// Root length weights (1, 2, 3 syllables).
    pub roots: [u32; 3],
    /// Stress rule.
    pub stress: Stress,
    /// Vowel used to break clusters.
    pub epenthetic: Ph,
}

/// Parses `"t4 k2"` into weighted phonemes (weight 1 when omitted).
pub(crate) fn weighted(s: &str) -> Vec<(Ph, u32)> {
    s.split_whitespace()
        .filter_map(|t| {
            let code = t.trim_end_matches(|c: char| c.is_ascii_digit());
            let w = t[code.len()..].parse().unwrap_or(1);
            Ph::from_code(code).map(|p| (p, w))
        })
        .collect()
}

/// Parses `"s+t t+r"` into sequences.
pub(crate) fn sequences(s: &str) -> Vec<Vec<Ph>> {
    s.split_whitespace()
        .map(|t| t.split('+').filter_map(Ph::from_code).collect::<Vec<_>>())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Parses `"a|e+n|"` into alternatives (empty alternatives kept).
pub(crate) fn alternatives(s: &str) -> Vec<Vec<Ph>> {
    s.split('|')
        .map(|t| {
            t.split('+')
                .filter_map(|c| Ph::from_code(c.trim()))
                .collect()
        })
        .collect()
}

impl Phonology {
    /// Builds the phonology of `spec`, varied by `rng`: some consonants are
    /// dropped and all weights are jittered.
    pub(crate) fn from_spec(spec: &Spec, rng: &mut Rng) -> Self {
        let mut consonants = weighted(spec.consonants);
        let keep_min = 9.min(consonants.len());
        let mut kept = Vec::with_capacity(consonants.len());
        for (i, &(p, w)) in consonants.iter().enumerate() {
            let left = consonants.len() - i;
            if kept.len() + left > keep_min && rng.chance(spec.drop) {
                continue;
            }
            kept.push((p, jitter(w, rng)));
        }
        consonants = kept;
        let has = |p: &Ph| consonants.iter().any(|(c, _)| c == p);
        let vowels = weighted(spec.vowels)
            .into_iter()
            .map(|(p, w)| (p, jitter(w, rng)))
            .collect();
        let pair = |v: &Vec<Ph>| v.len() == 2;
        let diphthongs = sequences(spec.diphthongs)
            .into_iter()
            .filter(pair)
            .map(|v| (v[0], v[1]))
            .collect();
        let clusters = sequences(spec.clusters)
            .into_iter()
            .filter(|c| c.iter().all(&has))
            .collect();
        let codas = weighted(spec.codas)
            .into_iter()
            .filter(|(p, _)| has(p))
            .map(|(p, w)| (p, jitter(w, rng)))
            .collect();
        let coda_clusters = sequences(spec.coda_clusters)
            .into_iter()
            .filter(|c| c.iter().all(&has))
            .collect();
        Self {
            consonants,
            vowels,
            diphthongs,
            clusters,
            codas,
            coda_clusters,
            no_initial: weighted(spec.no_initial)
                .into_iter()
                .map(|(p, _)| p)
                .collect(),
            initial_onset: spec.initial_onset,
            cluster_rate: spec.cluster_rate,
            medial_coda: spec.medial_coda,
            final_coda: spec.final_coda,
            diph_rate: spec.diph_rate,
            hiatus: spec.hiatus,
            harmony: spec.harmony,
            geminates: spec.geminates,
            max_medial: usize::from(spec.max_medial).clamp(1, MAX_RUN),
            roots: spec.roots,
            stress: spec.stress,
            epenthetic: Ph::code(spec.epenthetic),
        }
    }

    /// Consonant in the inventory.
    #[must_use]
    pub fn has_consonant(&self, p: Ph) -> bool {
        self.consonants.iter().any(|&(c, _)| c == p)
    }

    /// Valid onset (empty allowed).
    #[must_use]
    pub fn is_onset(&self, s: &[Ph]) -> bool {
        match s.len() {
            0 => true,
            1 => self.has_consonant(s[0]),
            _ => self.clusters.iter().any(|c| c.as_slice() == s),
        }
    }

    /// Valid coda (empty allowed).
    #[must_use]
    pub fn is_coda(&self, s: &[Ph]) -> bool {
        match s.len() {
            0 => true,
            1 => self.codas.iter().any(|&(c, _)| c == s[0]),
            _ => self.coda_clusters.iter().any(|c| c.as_slice() == s),
        }
    }

    fn geminate_ok(&self, p: Ph) -> bool {
        let code = p.info().code;
        self.geminates && code.len() == 1 && code != "'" && !matches!(code, "h" | "w" | "y")
    }

    /// Valid word-medial consonant run.
    #[must_use]
    pub fn medial_ok(&self, run: &[Ph]) -> bool {
        if run.len() > self.max_medial {
            return false;
        }
        for w in run.windows(2) {
            if w[0] == w[1] && !(self.geminate_ok(w[0]) && run.len() == 2) {
                return false;
            }
            if w[1] == Ph::code("h") {
                return false;
            }
        }
        if run.len() == 2 && run[0] == run[1] {
            return true;
        }
        (0..run.len()).any(|k| self.is_coda(&run[..k]) && self.is_onset(&run[k..]))
    }

    /// Valid vowel run.
    #[must_use]
    pub fn vowels_ok(&self, run: &[Ph]) -> bool {
        match run {
            [_] => true,
            [a, b] => {
                a != b
                    && (self.diphthongs.contains(&(*a, *b))
                        || (self.hiatus && !a.info().long && !b.info().long))
            }
            _ => false,
        }
    }

    /// Checks every phonotactic rule (not harmony, which binds roots only).
    #[must_use]
    pub fn valid(&self, w: &[Ph]) -> bool {
        self.first_violation(w).is_none()
    }

    /// Roots also obey vowel harmony.
    #[must_use]
    pub fn valid_root(&self, w: &[Ph]) -> bool {
        self.valid(w) && (!self.harmony || harmonic(w))
    }

    /// Index span `(start, end, kind)` of the first rule broken, if any.
    pub(crate) fn first_violation(&self, w: &[Ph]) -> Option<(usize, usize, Fault)> {
        let runs = runs(w);
        if !runs.iter().any(|r| r.2) {
            return Some((0, w.len(), Fault::NoVowel));
        }
        let last = runs.len().saturating_sub(1);
        for (n, &(s, e, vowel)) in runs.iter().enumerate() {
            let seg = &w[s..e];
            if vowel {
                if !self.vowels_ok(seg) {
                    return Some((s, e, Fault::Vowels));
                }
            } else if n == 0 {
                let banned = self.no_initial.contains(&seg[0]);
                if banned || !self.is_onset(seg) {
                    return Some((s, e, Fault::Initial));
                }
            } else if n == last {
                if !self.is_coda(seg) {
                    return Some((s, e, Fault::Final));
                }
            } else if !self.medial_ok(seg) {
                return Some((s, e, Fault::Medial));
            }
        }
        None
    }

    /// Syllable start indices (generic maximal-onset syllabification).
    #[must_use]
    pub fn syllables(&self, w: &[Ph]) -> Vec<usize> {
        let runs = runs(w);
        let mut starts = Vec::new();
        for (n, &(s, e, vowel)) in runs.iter().enumerate() {
            if vowel {
                if starts.is_empty() {
                    starts.push(0);
                }
                if e - s == 2 && !self.diphthongs.contains(&(w[s], w[s + 1])) {
                    starts.push(s + 1);
                }
                continue;
            }
            if n == 0 || n + 1 == runs.len() {
                continue;
            }
            let run = &w[s..e];
            let k = (0..run.len())
                .find(|&k| self.is_coda(&run[..k]) && self.is_onset(&run[k..]))
                .unwrap_or(run.len().saturating_sub(1));
            starts.push(s + k);
        }
        if starts.is_empty() {
            starts.push(0);
        }
        starts
    }

    /// Index of the stressed syllable among `count`.
    #[must_use]
    pub fn stressed(&self, count: usize) -> usize {
        match self.stress {
            Stress::Initial => 0,
            Stress::Penult => count.saturating_sub(2),
            Stress::Final => count.saturating_sub(1),
        }
    }

    fn pick(rng: &mut Rng, items: &[(Ph, u32)]) -> Option<Ph> {
        let w: Vec<u32> = items.iter().map(|x| x.1).collect();
        items.get(rng.weighted(&w)).map(|x| x.0)
    }

    fn vowel(&self, rng: &mut Rng, class: Option<bool>) -> Ph {
        let pool: Vec<(Ph, u32)> = self
            .vowels
            .iter()
            .copied()
            .filter(|&(v, _)| {
                class.is_none() || front_class(v).is_none() || front_class(v) == class
            })
            .collect();
        Self::pick(rng, &pool).unwrap_or(self.epenthetic)
    }

    /// One random syllable.
    fn syllable(&self, rng: &mut Rng, first: bool, last: bool, class: Option<bool>) -> Vec<Ph> {
        let mut out = Vec::with_capacity(5);
        let onset = !first || rng.chance(self.initial_onset);
        if onset {
            let rate = if first {
                self.cluster_rate
            } else {
                self.cluster_rate / 4
            };
            if !self.clusters.is_empty() && rng.chance(rate) {
                if let Some(c) = rng.pick(&self.clusters) {
                    out.extend_from_slice(c);
                }
            } else {
                let pool: Vec<(Ph, u32)> = self
                    .consonants
                    .iter()
                    .copied()
                    .filter(|(p, _)| !first || !self.no_initial.contains(p))
                    .collect();
                if let Some(p) = Self::pick(rng, &pool) {
                    out.push(p);
                }
            }
        }
        let v = self.vowel(rng, class);
        out.push(v);
        if !v.info().long && rng.chance(self.diph_rate) {
            let options: Vec<Ph> = self
                .diphthongs
                .iter()
                .filter(|d| d.0 == v)
                .map(|d| d.1)
                .collect();
            if let Some(&d) = rng.pick(&options) {
                out.push(d);
            }
        }
        let closed = rng.chance(if last {
            self.final_coda
        } else {
            self.medial_coda
        });
        if closed {
            if last && !self.coda_clusters.is_empty() && rng.chance(self.cluster_rate) {
                if let Some(c) = rng.pick(&self.coda_clusters) {
                    out.extend_from_slice(c);
                }
            } else if let Some(p) = Self::pick(rng, &self.codas) {
                out.push(p);
            }
        }
        out
    }

    /// A random word of `syllables` syllables obeying every rule; retries,
    /// then falls back to a plain CV shape.
    pub(crate) fn word(&self, rng: &mut Rng, syllables: usize) -> Vec<Ph> {
        let n = syllables.max(1);
        for _ in 0..48 {
            let class = self.harmony.then(|| rng.chance(500));
            let mut w = Vec::with_capacity(n * 3);
            for i in 0..n {
                w.extend(self.syllable(rng, i == 0, i + 1 == n, class));
            }
            if self.valid_root(&w) {
                return w;
            }
        }
        let c = Self::pick(rng, &self.consonants).unwrap_or(Ph::code("n"));
        let mut w = Vec::new();
        for _ in 0..n {
            w.push(c);
            w.push(self.vowel(rng, None));
        }
        w
    }

    /// A short affix: one open or lightly closed syllable, no clusters.
    pub(crate) fn affix(&self, rng: &mut Rng) -> Vec<Ph> {
        for _ in 0..32 {
            let mut w = Vec::with_capacity(3);
            if rng.chance(550) {
                let pool: Vec<(Ph, u32)> = self
                    .consonants
                    .iter()
                    .copied()
                    .filter(|(p, _)| !self.no_initial.contains(p))
                    .collect();
                if let Some(p) = Self::pick(rng, &pool) {
                    w.push(p);
                }
            }
            w.push(self.vowel(rng, self.harmony.then_some(false)));
            if rng.chance(self.final_coda / 2) {
                if let Some(p) = Self::pick(rng, &self.codas) {
                    w.push(p);
                }
            }
            if self.valid(&w) {
                return w;
            }
        }
        vec![self.epenthetic]
    }

    /// Number of syllables for a new root.
    pub(crate) fn root_len(&self, rng: &mut Rng) -> usize {
        rng.weighted(&self.roots) + 1
    }

    /// Nearest inventory consonant to `p` (for loanwords and repairs).
    #[must_use]
    pub fn nearest_consonant(&self, p: Ph, codas_only: bool) -> Ph {
        let pool: Vec<Ph> = if codas_only {
            self.codas.iter().map(|x| x.0).collect()
        } else {
            self.consonants.iter().map(|x| x.0).collect()
        };
        pool.into_iter()
            .min_by_key(|&c| (p.distance(c), c.0))
            .unwrap_or(p)
    }

    /// Nearest inventory vowel to `p`.
    #[must_use]
    pub fn nearest_vowel(&self, p: Ph) -> Ph {
        self.vowels
            .iter()
            .map(|x| x.0)
            .min_by_key(|&v| (p.distance(v), v.0))
            .unwrap_or(self.epenthetic)
    }
}

/// What rule a word breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    NoVowel,
    Vowels,
    Initial,
    Final,
    Medial,
}

/// Maximal runs `(start, end, is_vowel)`.
pub(crate) fn runs(w: &[Ph]) -> Vec<(usize, usize, bool)> {
    let mut out: Vec<(usize, usize, bool)> = Vec::new();
    for (i, p) in w.iter().enumerate() {
        let v = p.is_vowel();
        match out.last_mut() {
            Some(last) if last.2 == v => last.1 = i + 1,
            _ => out.push((i, i + 1, v)),
        }
    }
    out
}

/// Integer jitter of a weight to 70–130 %.
fn jitter(w: u32, rng: &mut Rng) -> u32 {
    let pct = 70 + u32::try_from(rng.below(61)).unwrap_or(30);
    (w * pct).div_ceil(100).max(1)
}
