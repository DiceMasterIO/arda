//! A naming language: phonology, spelling, lexicon, grammar and history.

use crate::change::{tidy, SoundChange};
use crate::filter::Blocklist;
use crate::meaning::{Class, Meaning, ALL};
use crate::ortho::{capitalise, Ortho};
use crate::phoneme::Ph;
use crate::phonology::{alternatives, Phonology};
use crate::preset::{Preset, FAMILY_STYLES};
use crate::rng::{hash_str, Rng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Salt that derives the substrate seed from a language seed.
const SUBSTRATE: u64 = 0x5AB5_7A7E;
/// Meaningless roots kept for personal names.
const NAME_ROOTS: usize = 48;

/// Root shapes the lexicon draws.
#[derive(Clone, Copy)]
enum Shape {
    Affix,
    Head,
    Root,
    Name,
}

/// Naming customs of a language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grammar {
    /// Compounds put the head first ("Ford-of-Oaks") rather than last.
    pub head_first: bool,
    /// Head-first compounds join nouns with a genitive linker.
    pub linker: bool,
    /// Family-name style weights (see [`crate::preset::FAMILY_STYLES`]).
    pub family: [u32; 5],
    /// Per mille of adults with a byname.
    pub byname: u32,
}

/// Options beyond seed and preset.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Seed of the substrate tongue; share it between languages of one
    /// world so their old river names agree. Defaults to one derived from
    /// the language seed.
    pub substrate_seed: Option<u64>,
    /// Blocklist to use instead of [`Blocklist::builtin`].
    pub blocklist: Option<Blocklist>,
}

/// A complete naming language, deterministic in `(seed, preset)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Language {
    /// Preset it was built from.
    pub preset: Preset,
    /// Seed it was built from.
    #[serde(with = "crate::ids")]
    pub seed: u64,
    /// Sounds and phonotactics.
    pub phonology: Phonology,
    /// Spelling.
    pub ortho: Ortho,
    /// Customs.
    pub grammar: Grammar,
    /// One root per meaning, in [`crate::meaning::ALL`] order.
    pub lexicon: Vec<Vec<Ph>>,
    /// Meaningless roots for personal names.
    pub name_roots: Vec<Vec<Ph>>,
    /// Feminine and masculine given-name endings.
    pub endings: [Vec<Vec<Ph>>; 2],
    /// Dialect sound changes applied when a word is spoken, in order.
    pub changes: Vec<SoundChange>,
    /// The older tongue under this one (rivers, some mountains).
    pub substrate: Option<Box<Language>>,
    /// Words no name may contain.
    pub blocklist: Blocklist,
}

impl Language {
    /// Builds the language for `seed` in the style of `preset`.
    #[must_use]
    pub fn new(seed: u64, preset: Preset) -> Self {
        Self::with_options(seed, preset, &Options::default())
    }

    /// Builds with explicit options.
    #[must_use]
    pub fn with_options(seed: u64, preset: Preset, opts: &Options) -> Self {
        let blocklist = opts
            .blocklist
            .clone()
            .unwrap_or_else(|| Blocklist::builtin().clone());
        let mut lang = Self::bare(seed, preset, blocklist.clone());
        if preset != Preset::Ancient {
            let sub_seed = opts.substrate_seed.unwrap_or(seed ^ SUBSTRATE);
            lang.substrate = Some(Box::new(Self::bare(sub_seed, Preset::Ancient, blocklist)));
        }
        lang
    }

    fn bare(seed: u64, preset: Preset, blocklist: Blocklist) -> Self {
        let spec = preset.spec();
        let mut rng = Rng::new(seed, &[hash_str(spec.name)]);
        let phonology = Phonology::from_spec(spec, &mut rng);
        let head_first = rng.chance(spec.head_first);
        let grammar = Grammar {
            head_first,
            linker: head_first && rng.chance(spec.linker),
            family: spec.family,
            byname: spec.byname,
        };
        let mut lang = Self {
            preset,
            seed,
            phonology,
            ortho: Ortho::new(spec.ortho, spec.final_spell, spec.k_front),
            grammar,
            lexicon: Vec::with_capacity(ALL.len()),
            name_roots: Vec::with_capacity(NAME_ROOTS),
            endings: [alternatives(spec.fem), alternatives(spec.masc)],
            changes: Vec::new(),
            substrate: None,
            blocklist,
        };
        lang.build_lexicon(&mut rng);
        lang
    }

    fn build_lexicon(&mut self, rng: &mut Rng) {
        let mut seen = BTreeSet::new();
        for &m in ALL {
            let shape = match m.info().class {
                Class::Affix => Shape::Affix,
                Class::Head => Shape::Head,
                _ => Shape::Root,
            };
            let w = self.fresh_root(rng, &mut seen, shape);
            self.lexicon.push(w);
        }
        for _ in 0..NAME_ROOTS {
            let w = self.fresh_root(rng, &mut seen, Shape::Name);
            self.name_roots.push(w);
        }
    }

    /// A root whose spelling is new and not blocked.
    fn fresh_root(&self, rng: &mut Rng, seen: &mut BTreeSet<String>, shape: Shape) -> Vec<Ph> {
        let mut w = Vec::new();
        for attempt in 0..64 {
            let longer = usize::from(attempt >= 40);
            let len = self.phonology.root_len(rng);
            w = match shape {
                Shape::Affix => self.phonology.affix(rng),
                // Heads work like suffixes: mostly one syllable.
                Shape::Head => {
                    let n = if rng.chance(750) { 1 } else { len.min(2) };
                    self.phonology.word(rng, n + longer)
                }
                Shape::Root => self.phonology.word(rng, len + longer),
                Shape::Name => self.phonology.word(rng, len.min(2) + longer),
            };
            let s = self.ortho.render(&w);
            let min = match shape {
                Shape::Affix => 1,
                Shape::Name => 3,
                _ => 2,
            };
            if s.len() >= min && !seen.contains(&s) && !self.blocklist.blocks(&s) {
                seen.insert(s);
                return w;
            }
        }
        w
    }

    /// Underlying root of a meaning.
    #[must_use]
    pub fn root(&self, m: Meaning) -> &[Ph] {
        self.lexicon.get(m as usize).map_or(&[], Vec::as_slice)
    }

    /// A meaning's word as spoken and spelled in this language (dialect
    /// changes included).
    #[must_use]
    pub fn word(&self, m: Meaning) -> String {
        self.spell(&self.speak(self.root(m)))
    }

    /// Applies this dialect's sound changes.
    #[must_use]
    pub fn speak(&self, w: &[Ph]) -> Vec<Ph> {
        if self.changes.is_empty() {
            return w.to_vec();
        }
        let mut out = w.to_vec();
        for c in &self.changes {
            out = c.apply(&out);
        }
        tidy(&mut out, self.phonology.epenthetic);
        out
    }

    /// Capitalised spelling.
    #[must_use]
    pub fn spell(&self, w: &[Ph]) -> String {
        capitalise(&self.ortho.render(w))
    }

    /// Respelling with stress.
    #[must_use]
    pub fn pronounce(&self, w: &[Ph]) -> String {
        let starts = self.phonology.syllables(w);
        let stressed = self.phonology.stressed(starts.len());
        self.ortho.respell(w, &starts, stressed)
    }

    /// A copy of this language with extra sound changes (a dialect).
    #[must_use]
    pub fn with_changes(&self, changes: &[SoundChange]) -> Self {
        let mut d = self.clone();
        d.changes.extend_from_slice(changes);
        d
    }

    /// Family-style weights in [`FAMILY_STYLES`] order, zipped.
    pub(crate) fn family_weights(&self) -> Vec<(crate::preset::FamilyStyle, u32)> {
        FAMILY_STYLES
            .iter()
            .copied()
            .zip(self.grammar.family)
            .collect()
    }
}
