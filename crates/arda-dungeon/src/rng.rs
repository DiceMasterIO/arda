//! A tiny deterministic RNG (SplitMix64), so output never depends on a
//! third-party generator's version.

/// SplitMix64 stream.
#[derive(Debug, Clone)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(arda_tactical::noise::mix(seed ^ 0xD0_6E0B))
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        arda_tactical::noise::mix(self.0)
    }

    /// Uniform in `[0, 1)`.
    pub(crate) fn f(&mut self) -> f32 {
        arda_tactical::noise::unit(self.next_u64())
    }

    /// True with probability `p`.
    pub(crate) fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }

    /// Uniform integer in `lo..=hi` (`lo` when `hi < lo`).
    pub(crate) fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        let span = u64::from(hi - lo) + 1;
        lo + u32::try_from(self.next_u64() % span).unwrap_or(0)
    }

    /// A uniform index below `n` (`n` > 0).
    pub(crate) fn index(&mut self, n: usize) -> usize {
        usize::try_from(self.next_u64() % (n.max(1) as u64)).unwrap_or(0)
    }

    /// Fisher-Yates shuffle.
    pub(crate) fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = self.index(i + 1);
            xs.swap(i, j);
        }
    }
}
