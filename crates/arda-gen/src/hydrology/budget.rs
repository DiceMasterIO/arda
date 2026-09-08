//! Checked annual forcing and physical-width errors.
use thiserror::Error;
/// Invalid annual forcing or arithmetic outside its declared integer domain.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BudgetError {
    /// A checked product, sum or conversion overflowed.
    #[error("water arithmetic overflow: {0}")]
    Overflow(&'static str),
    /// A supplied forcing value violated its declared units or range.
    #[error("invalid water forcing: {0}")]
    InvalidForcing(&'static str),
}
