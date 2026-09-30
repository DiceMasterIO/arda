//! A rectangle of values addressed by global square (or vertex) coordinates.

/// A `usize` count as a signed coordinate span.
#[must_use]
pub fn span(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Values over `[x0, x0 + w) × [y0, y0 + h)` in global coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid<T> {
    /// First global column.
    pub x0: i64,
    /// First global row.
    pub y0: i64,
    /// Columns.
    pub w: usize,
    /// Rows.
    pub h: usize,
    /// Row-major values.
    pub data: Vec<T>,
}

impl<T: Clone> Grid<T> {
    /// A grid filled with one value.
    #[must_use]
    pub fn new(x0: i64, y0: i64, w: usize, h: usize, fill: T) -> Self {
        Self {
            x0,
            y0,
            w,
            h,
            data: vec![fill; w * h],
        }
    }

    /// Row-major index of global `(x, y)`, if inside.
    #[must_use]
    pub fn index(&self, x: i64, y: i64) -> Option<usize> {
        let i = usize::try_from(x - self.x0).ok()?;
        let j = usize::try_from(y - self.y0).ok()?;
        (i < self.w && j < self.h).then_some(j * self.w + i)
    }

    /// The value at global `(x, y)`, if inside.
    #[must_use]
    pub fn get(&self, x: i64, y: i64) -> Option<&T> {
        self.index(x, y).and_then(|i| self.data.get(i))
    }

    /// The value at global `(x, y)`, clamped to the grid.
    #[must_use]
    pub fn clamped(&self, x: i64, y: i64) -> &T {
        let i = (x - self.x0).clamp(0, span(self.w) - 1);
        let j = (y - self.y0).clamp(0, span(self.h) - 1);
        let k = usize::try_from(j).unwrap_or(0) * self.w + usize::try_from(i).unwrap_or(0);
        &self.data[k.min(self.data.len().saturating_sub(1))]
    }

    /// Mutable value at global `(x, y)`, if inside.
    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut T> {
        self.index(x, y).and_then(|i| self.data.get_mut(i))
    }

    /// Global coordinates of every cell, row-major.
    pub fn coords(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        let (x0, y0, w) = (self.x0, self.y0, span(self.w));
        (0..span(self.h)).flat_map(move |j| (0..w).map(move |i| (x0 + i, y0 + j)))
    }
}
