//! Bounded witnessed MST production between routing and basin hierarchy stages.
use super::{hierarchy, routing, saddles};
pub(crate) mod accepted_sort;
pub(crate) mod io;
pub(crate) mod producer;
pub(crate) mod slots;

pub use accepted_sort::{AcceptedSorter, EdgeReader, ReadLimits, SortError, SortLimits, SortWork};
pub use io::{IoWork, StageError};
pub use producer::{produce, MinimumReader, MstError, MstLimits, MstProduct, Work};
pub use slots::SlotWork;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
