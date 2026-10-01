//! The two work lists every WFC here keeps: the propagation queue (cells
//! whose neighbours changed) and the collapse frontier (undecided cells,
//! fewest options first, ties by a hashed priority so there is no
//! scan-order bias). `arda-refine`'s corner solver uses the same lists.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// A FIFO of cells, each queued at most once.
#[derive(Debug, Clone)]
pub struct WorkQueue {
    queue: VecDeque<usize>,
    queued: Vec<bool>,
}

impl WorkQueue {
    /// An empty queue over `n` cells.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            queued: vec![false; n],
        }
    }

    /// Queues `i` unless it is already queued.
    pub fn push(&mut self, i: usize) {
        if let Some(q) = self.queued.get_mut(i) {
            if !*q {
                *q = true;
                self.queue.push_back(i);
            }
        }
    }

    /// The next cell, in queue order.
    pub fn pop(&mut self) -> Option<usize> {
        let i = self.queue.pop_front()?;
        if let Some(q) = self.queued.get_mut(i) {
            *q = false;
        }
        Some(i)
    }

    /// Drops every queued cell.
    pub fn clear(&mut self) {
        self.queue.clear();
        self.queued.iter_mut().for_each(|q| *q = false);
    }
}

/// Undecided cells ordered by `(options, priority, index)`, smallest first.
/// Entries go stale when a cell's options change; callers skip an entry
/// whose count no longer matches.
#[derive(Debug, Clone, Default)]
pub struct Frontier {
    heap: BinaryHeap<Reverse<(u32, u32, usize)>>,
}

impl Frontier {
    /// An empty frontier.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds cell `i` with `count` options, if it is still undecided.
    pub fn push(&mut self, count: u32, prio: u32, i: usize) {
        if count > 1 {
            self.heap.push(Reverse((count, prio, i)));
        }
    }

    /// The smallest entry `(count, priority, index)`.
    pub fn pop(&mut self) -> Option<(u32, u32, usize)> {
        self.heap.pop().map(|Reverse(e)| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_holds_each_cell_once() {
        let mut q = WorkQueue::new(4);
        q.push(2);
        q.push(2);
        q.push(1);
        assert_eq!(q.pop(), Some(2));
        q.push(2);
        assert_eq!((q.pop(), q.pop(), q.pop()), (Some(1), Some(2), None));
    }

    #[test]
    fn frontier_orders_by_count_then_priority() {
        let mut f = Frontier::new();
        f.push(3, 1, 0);
        f.push(2, 9, 1);
        f.push(2, 4, 2);
        f.push(1, 0, 3);
        assert_eq!(f.pop(), Some((2, 4, 2)));
        assert_eq!(f.pop(), Some((2, 9, 1)));
        assert_eq!(f.pop(), Some((3, 1, 0)));
        assert_eq!(f.pop(), None);
    }
}
