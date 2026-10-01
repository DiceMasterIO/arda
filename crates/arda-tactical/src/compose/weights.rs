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
