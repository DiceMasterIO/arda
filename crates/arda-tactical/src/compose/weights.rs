//! Ground blend weights: up to four keys per pixel (goal 62).

/// Blend weights at a pixel: up to four `(key, weight)` pairs.
#[derive(Clone, Copy, Default)]
pub(super) struct Weights {
    pub(super) n: usize,
    pub(super) k: [(usize, f32); 4],
}

impl Weights {
    pub(super) fn one(key: usize) -> Self {
        Self {
            n: 1,
            k: [(key, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)],
        }
    }

    /// The share of the weight on keys for which `pick` holds.
    pub(super) fn share(&self, pick: impl Fn(usize) -> bool) -> f32 {
        let (mut on, mut all) = (0.0, 0.0);
        for e in &self.k[..self.n] {
            all += e.1;
            if pick(e.0) {
                on += e.1;
            }
        }
        on / all.max(1e-6)
    }

    pub(super) fn add(&mut self, key: usize, w: f32) {
        for e in &mut self.k[..self.n] {
            if e.0 == key {
                e.1 += w;
                return;
            }
        }
        if self.n < 4 {
            self.k[self.n] = (key, w);
            self.n += 1;
        }
    }
}
