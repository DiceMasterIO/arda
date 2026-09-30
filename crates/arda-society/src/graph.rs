//! The road network as a settlement graph, with bounded shortest paths.

use crate::input::{Road, RoadClass, WorldSettlements};
use crate::num::dist_m;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// One road between two settlements, seen from one end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// Neighbour node index.
    pub to: usize,
    /// Road id.
    pub road: u64,
    /// Road length, metres.
    pub len_m: u64,
    /// Road class.
    pub class: RoadClass,
}

/// Shortest paths from one source, within a cutoff.
#[derive(Debug, Clone, Default)]
pub struct Reach {
    /// `(node, distance_m)` for every reached node, source included, by node.
    pub dist: BTreeMap<usize, u64>,
    /// `node → (previous node, road id)` on the shortest path.
    pub prev: BTreeMap<usize, (usize, u64)>,
}

impl Reach {
    /// Nodes on the path from the source to `target`, source first.
    #[must_use]
    pub fn nodes_to(&self, target: usize) -> Vec<usize> {
        let mut out = vec![target];
        let mut cur = target;
        while let Some(&(p, _)) = self.prev.get(&cur) {
            out.push(p);
            cur = p;
        }
        out.reverse();
        out
    }

    /// Road ids on the path from the source to `target`, in travel order.
    #[must_use]
    pub fn roads_to(&self, target: usize) -> Vec<u64> {
        let mut out = Vec::new();
        let mut cur = target;
        while let Some(&(p, r)) = self.prev.get(&cur) {
            out.push(r);
            cur = p;
        }
        out.reverse();
        out
    }
}

/// Settlement graph built from roads that join two settlements.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    /// Settlement id per node, in input order.
    pub ids: Vec<u64>,
    /// Node index per settlement id.
    pub index: BTreeMap<u64, usize>,
    /// Adjacency, sorted by `(to, road)`.
    pub adj: Vec<Vec<Edge>>,
}

impl Graph {
    /// Builds the graph; roads to a map edge or to unknown ids are skipped.
    #[must_use]
    pub fn build(world: &WorldSettlements) -> Self {
        let ids: Vec<u64> = world.settlements.iter().map(|s| s.id).collect();
        let index: BTreeMap<u64, usize> = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        let mut adj = vec![Vec::new(); ids.len()];
        for road in &world.roads {
            let Some((a, b)) = ends(road, &index) else {
                continue;
            };
            if a == b {
                continue;
            }
            let len_m = road_len(road, world, a, b);
            adj[a].push(Edge {
                to: b,
                road: road.id,
                len_m,
                class: road.class,
            });
            adj[b].push(Edge {
                to: a,
                road: road.id,
                len_m,
                class: road.class,
            });
        }
        for list in &mut adj {
            list.sort_by_key(|e| (e.to, e.road));
        }
        Self { ids, index, adj }
    }

    /// Number of settlements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the graph has no settlements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Dijkstra from `src`, visiting nodes up to `cutoff_m` away. Ties go to
    /// the lower node index, so the result is deterministic.
    #[must_use]
    pub fn reach(&self, src: usize, cutoff_m: u64) -> Reach {
        let mut r = Reach::default();
        let mut heap = BinaryHeap::new();
        r.dist.insert(src, 0);
        heap.push(Reverse((0_u64, src)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if r.dist.get(&u).is_some_and(|&best| d > best) {
                continue;
            }
            let Some(edges) = self.adj.get(u) else {
                continue;
            };
            for e in edges {
                let nd = d.saturating_add(e.len_m);
                if nd > cutoff_m {
                    continue;
                }
                let better = r.dist.get(&e.to).is_none_or(|&cur| nd < cur);
                if better {
                    r.dist.insert(e.to, nd);
                    r.prev.insert(e.to, (u, e.road));
                    heap.push(Reverse((nd, e.to)));
                }
            }
        }
        r
    }

    /// Dijkstra from `src` that stops once `k` other settlements are
    /// settled (or `cutoff_m` is passed): the `k` nearest markets by road.
    /// Only settled nodes are kept, so a reach holds at most `k + 1`
    /// entries whatever the network's size; ties go to the lower node
    /// index, as in [`Graph::reach`].
    #[must_use]
    pub fn reach_nearest(&self, src: usize, cutoff_m: u64, k: usize) -> Reach {
        let mut r = Reach::default();
        let mut done: BTreeMap<usize, u64> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        r.dist.insert(src, 0);
        heap.push(Reverse((0_u64, src)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if done.contains_key(&u) || r.dist.get(&u).is_some_and(|&best| d > best) {
                continue;
            }
            done.insert(u, d);
            if done.len() > k {
                break;
            }
            let Some(edges) = self.adj.get(u) else {
                continue;
            };
            for e in edges {
                let nd = d.saturating_add(e.len_m);
                if nd > cutoff_m || done.contains_key(&e.to) {
                    continue;
                }
                if r.dist.get(&e.to).is_none_or(|&cur| nd < cur) {
                    r.dist.insert(e.to, nd);
                    r.prev.insert(e.to, (u, e.road));
                    heap.push(Reverse((nd, e.to)));
                }
            }
        }
        r.prev.retain(|n, _| done.contains_key(n));
        r.dist = done;
        r
    }

    /// Connected components as lists of node indices, each sorted.
    #[must_use]
    pub fn components(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.len()];
        let mut out = Vec::new();
        for start in 0..self.len() {
            if seen[start] {
                continue;
            }
            let mut comp = Vec::new();
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(u) = stack.pop() {
                comp.push(u);
                for e in &self.adj[u] {
                    if !seen[e.to] {
                        seen[e.to] = true;
                        stack.push(e.to);
                    }
                }
            }
            comp.sort_unstable();
            out.push(comp);
        }
        out
    }
}

fn ends(road: &Road, index: &BTreeMap<u64, usize>) -> Option<(usize, usize)> {
    let a = *index.get(&road.from)?;
    let b = *index.get(&road.to?)?;
    Some((a, b))
}

fn road_len(road: &Road, world: &WorldSettlements, a: usize, b: usize) -> u64 {
    if road.length_m > 0 {
        return road.length_m;
    }
    let (sa, sb) = (&world.settlements[a], &world.settlements[b]);
    dist_m(sa.x_m, sa.y_m, sb.x_m, sb.y_m).max(1)
}
