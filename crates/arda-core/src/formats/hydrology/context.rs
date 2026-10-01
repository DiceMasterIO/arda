//! Standalone area hydrology contexts: bounded encode and decode, and
//! validation against the modeled domain.

use super::*;
use crate::GlobalCell;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
/// Caller-selected hard limits for standalone area context decoding and encoding.
pub struct ContextLimits {
    /// Maximum encoded bytes accepted for one context.
    pub max_bytes: u64,
    /// Maximum total count across the four logical context tables.
    pub max_records: u32,
}
impl Default for ContextLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_records: 1_048_576,
        }
    }
}
fn ordered<T: FixedRecord>(values: &[T]) -> Result<()> {
    for pair in values.windows(2) {
        require(
            pair[0].key() < pair[1].key(),
            "context records are not unique ascending",
        )?;
    }
    Ok(())
}
fn validate_context(ctx: &AreaHydrologyContext) -> Result<()> {
    require(
        ctx.model_revision == MODEL_REVISION,
        "unsupported area hydrology model",
    )?;
    ordered(&ctx.lakes)?;
    ordered(&ctx.catchments)?;
    ordered(&ctx.crossings)?;
    ordered(&ctx.reaches)?;
    let owners: BTreeSet<_> = ctx.catchments.iter().map(|v| v.catchment).collect();
    let reaches: BTreeMap<_, _> = ctx.reaches.iter().map(|v| (v.id, v)).collect();
    for reach in &ctx.reaches {
        require(
            owners.contains(&reach.catchment),
            "reach lacks copied catchment",
        )?;
    }
    for crossing in &ctx.crossings {
        let reach = reaches
            .get(&crossing.reach)
            .ok_or(HydrologyFormatError::Invalid("crossing lacks copied reach"))?;
        require(
            owners.contains(&crossing.catchment)
                && reach.catchment == crossing.catchment
                && reach.drainage_cells == crossing.drainage_cells
                && reach.annual_volume == crossing.annual_volume
                && reach.mean_discharge == crossing.mean_discharge
                && reach.receiving == crossing.receiving,
            "crossing disagrees with copied reach",
        )?;
    }
    // Referenced basin/downstream identities may lie outside this bounded area copy.
    // The global indexed authority validates their existence, without recursive loading.
    Ok(())
}
const CONTEXT_HEADER_BYTES: usize = 28;
fn context_limits(counts: [u32; 4], limits: ContextLimits) -> Result<u64> {
    let widths = [
        GlobalLake::WIDTH,
        AnnualCatchment::WIDTH,
        SharedCrossing::WIDTH,
        GlobalReach::WIDTH,
    ];
    let total: u64 = counts.iter().map(|&v| u64::from(v)).sum();
    let bytes = counts.into_iter().zip(widths).try_fold(
        CONTEXT_HEADER_BYTES as u64,
        |sum, (count, width)| {
            sum.checked_add(
                u64::from(count)
                    .checked_mul(width as u64)
                    .ok_or(HydrologyFormatError::Overflow)?,
            )
            .ok_or(HydrologyFormatError::Overflow)
        },
    )?;
    if total > u64::from(limits.max_records) || bytes > limits.max_bytes {
        return Err(HydrologyFormatError::Limit(
            "area context exceeds declared limits",
        ));
    }
    Ok(bytes)
}
fn append_records<T: FixedRecord>(out: &mut Vec<u8>, records: &[T]) -> Result<()> {
    for record in records {
        out.extend(encode_record(record)?);
    }
    Ok(())
}
fn read_records<T: FixedRecord>(r: &mut Decoder<'_>, count: u32) -> Result<Vec<T>> {
    let mut records = Vec::with_capacity(count as usize);
    for _ in 0..count {
        records.push(decode_record(r.take(T::WIDTH)?)?);
    }
    Ok(records)
}
/// Encode a standalone area copy using the default bounded context limits.
pub fn encode_area_context(ctx: &AreaHydrologyContext) -> Result<Vec<u8>> {
    encode_area_context_with_limits(ctx, ContextLimits::default())
}
/// Encode a standalone area copy with explicit hard resource limits.
pub fn encode_area_context_with_limits(
    ctx: &AreaHydrologyContext,
    limits: ContextLimits,
) -> Result<Vec<u8>> {
    let count = |n| u32::try_from(n).map_err(|_| HydrologyFormatError::Overflow);
    let counts = [
        count(ctx.lakes.len())?,
        count(ctx.catchments.len())?,
        count(ctx.crossings.len())?,
        count(ctx.reaches.len())?,
    ];
    let bytes = context_limits(counts, limits)?;
    validate_context(ctx)?;
    let mut out = Encoder {
        bytes: Vec::with_capacity(
            usize::try_from(bytes).map_err(|_| HydrologyFormatError::Overflow)?,
        ),
    };
    out.bytes.extend(CONTEXT_MAGIC);
    ctx.model_revision.put(&mut out)?;
    for n in counts {
        n.put(&mut out)?;
    }
    append_records(&mut out.bytes, &ctx.lakes)?;
    append_records(&mut out.bytes, &ctx.catchments)?;
    append_records(&mut out.bytes, &ctx.crossings)?;
    append_records(&mut out.bytes, &ctx.reaches)?;
    require(
        out.bytes.len() as u64 == bytes,
        "context output size mismatch",
    )?;
    Ok(out.bytes)
}
/// Decode bounded area copies using the identical fixed global-record codecs.
pub fn decode_area_context(bytes: &[u8], limits: ContextLimits) -> Result<AreaHydrologyContext> {
    if bytes.len() as u64 > limits.max_bytes {
        return Err(HydrologyFormatError::Limit("area context byte limit"));
    }
    let mut r = Decoder::new(bytes);
    require(r.take(8)? == CONTEXT_MAGIC, "bad area hydrology magic")?;
    let model_revision = u32::get(&mut r)?;
    require(
        model_revision == MODEL_REVISION,
        "unsupported area hydrology model",
    )?;
    let mut counts = [0; 4];
    for count in &mut counts {
        *count = u32::get(&mut r)?;
    }
    require(
        context_limits(counts, limits)? == bytes.len() as u64,
        "context length disagrees with counts",
    )?;
    let ctx = AreaHydrologyContext {
        model_revision,
        lakes: read_records(&mut r, counts[0])?,
        catchments: read_records(&mut r, counts[1])?,
        crossings: read_records(&mut r, counts[2])?,
        reaches: read_records(&mut r, counts[3])?,
    };
    r.finish()?;
    validate_context(&ctx)?;
    Ok(ctx)
}

/// Validate copied geometry against the modeled rectangle after area decoding.
///
/// The caller may derive this rectangle from the validated manifest; no global
/// table or neighboring area reads are needed. Referenced record existence stays
/// the global authority's responsibility.
///
/// # Errors
/// Rejects coordinates outside the domain and exports away from its actual rim.
pub fn validate_area_context_domain(
    ctx: &AreaHydrologyContext,
    domain: HydrologyDomain,
) -> Result<()> {
    domain.check()?;
    let inside = |c: GlobalCell| {
        require(
            c.x < domain.width_cells && c.y < domain.height_cells,
            "copied hydrology coordinate leaves modeled domain",
        )
    };
    let rim = |c: GlobalCell| {
        require(
            c.x == 0 || c.y == 0 || c.x == domain.width_cells - 1 || c.y == domain.height_cells - 1,
            "domain-export endpoint is not on modeled boundary",
        )
    };
    let account = |a: ReceivingAccount| match a {
        ReceivingAccount::Reach(id) => inside(id.start()),
        ReceivingAccount::Junction(id) => inside(id.cell()),
        ReceivingAccount::Lake(_) | ReceivingAccount::Sea | ReceivingAccount::DomainExport => {
            Ok(())
        }
    };
    for lake in &ctx.lakes {
        if let Some(spill) = lake.outlet {
            validate_spill_domain(&spill, domain)?;
            account(spill.receiving)?;
        }
    }
    for owner in &ctx.catchments {
        inside(owner.terminal)?;
        account(owner.receiving)?;
        if let Some(spill) = owner.potential_spill {
            validate_spill_domain(&spill, domain)?;
            account(spill.receiving)?;
        } else if owner.receiving == ReceivingAccount::DomainExport {
            rim(owner.terminal)?;
        }
    }
    for reach in &ctx.reaches {
        inside(reach.from)?;
        inside(reach.to)?;
        account(reach.receiving)?;
        if reach.receiving == ReceivingAccount::DomainExport {
            rim(reach.to)?;
        }
    }
    for crossing in &ctx.crossings {
        inside(crossing.from)?;
        inside(crossing.to)?;
        inside(crossing.id.low)?;
        inside(crossing.id.high)?;
        account(crossing.receiving)?;
    }
    Ok(())
}

/// Validate the domain-dependent part after decoding against root metadata.
pub fn validate_spill_domain(spill: &SpillConnection, domain: HydrologyDomain) -> Result<()> {
    spill.check()?;
    domain.check()?;
    let inside = |c: GlobalCell| c.x < domain.width_cells && c.y < domain.height_cells;
    require(inside(spill.from), "spill source leaves modeled domain")?;
    match spill.to {
        Some(to) => require(inside(to), "spill receiver leaves modeled domain"),
        None => require(
            spill.from.x == 0
                || spill.from.y == 0
                || spill.from.x + 1 == domain.width_cells
                || spill.from.y + 1 == domain.height_cells,
            "domain-export spill is not on modeled boundary",
        ),
    }
}
