//! Final accounting of a filled hierarchy: representative lakes, leaf nets
//! and the exact annual water balance.

use super::*;

pub(super) fn finish(
    input: &AnnualInput<'_>,
    checked: &Checked,
    state: &[State],
    paid: &[bool],
    sea: u128,
    exterior: u128,
    meter: &mut Meter,
) -> Result<AnnualSolution> {
    let n = input.nodes.len();
    let mut leaf_account = vec_of(n, 0_usize)?;
    let mut wet_cost = vec_of(n, 0_u128)?;
    let mut marginal = vec_of(n, 0_u128)?;
    let mut lake_cells = vec_of(n, 0_u32)?;
    let mut lake_volume = vec_of(n, 0_u128)?;
    for (i, node) in input.nodes.iter().enumerate() {
        if !checked.children[i].is_empty() {
            continue;
        }
        let mut at = i;
        let mut account = i;
        while let Some(p) = checked.parent[at] {
            meter.tick()?;
            if (state[p].active || state[p].redirect.is_some())
                && state[p].height > input.nodes[p].birth.raw()
            {
                account = p;
            }
            at = p;
        }
        leaf_account[i] = account;
        if state[account].height <= node.floor.raw() {
            leaf_account[i] = i;
        }
    }
    let mut balance = AnnualWaterBalance {
        land_precipitation: Litres(checked.source_p),
        land_loss: Litres(checked.source_a),
        sea_outflow: Litres(sea),
        domain_outflow: Litres(exterior),
        ..AnnualWaterBalance::default()
    };
    for (j, band) in input.bands.iter().enumerate() {
        meter.tick()?;
        if !paid[j] {
            continue;
        }
        let o = checked.owner[j];
        let a = leaf_account[o];
        let h = state[a].height;
        if h <= band.bed.raw() {
            return Err(AnnualError::Invalid("paid band not geometrically wet"));
        }
        wet_cost[o] = add(
            wet_cost[o],
            band.open_water_evaporation.0 - band.effective_land_loss.0,
        )?;
        balance.land_precipitation.0 = sub(balance.land_precipitation.0, band.precipitation.0)?;
        balance.land_loss.0 = sub(balance.land_loss.0, band.effective_land_loss.0)?;
        balance.lake_precipitation.0 = add(balance.lake_precipitation.0, band.precipitation.0)?;
        balance.lake_evaporation.0 =
            add(balance.lake_evaporation.0, band.open_water_evaporation.0)?;
        lake_cells[a] = lake_cells[a]
            .checked_add(band.cells)
            .ok_or(AnnualError::Overflow)?;
        let depth = u128::try_from(i64::from(h) - i64::from(band.bed.raw()))
            .map_err(|_| AnnualError::Overflow)?;
        let v = depth
            .checked_mul(u128::from(band.cells))
            .and_then(|v| v.checked_mul(10_000))
            .ok_or(AnnualError::Overflow)?;
        lake_volume[a] = add(lake_volume[a], v)?;
    }
    let mut shares: Vec<(usize, u128, u128)> = Vec::new();
    shares
        .try_reserve_exact(input.bands.len())
        .map_err(|_| AnnualError::Limit("allocation"))?;
    for (i, s) in state.iter().enumerate() {
        if s.partial == 0 {
            continue;
        }
        let end = group_end(input, i, s.next, meter)?;
        let mut total = 0_u128;
        for band in &input.bands[s.next..end] {
            total = add(
                total,
                band.open_water_evaporation.0 - band.effective_land_loss.0,
            )?;
        }
        shares.clear();
        let mut allocated = 0_u128;
        for j in s.next..end {
            meter.tick()?;
            let d = input.bands[j].open_water_evaporation.0 - input.bands[j].effective_land_loss.0;
            let product = s.partial.checked_mul(d).ok_or(AnnualError::Overflow)?;
            let whole = product / total;
            shares.push((j, whole, product % total));
            allocated = add(allocated, whole)?;
        }
        // Counted insertion sort avoids an unmetered comparison path. Group size is
        // independently admitted; the work cap stops pathological same-height groups.
        for j in 1..shares.len() {
            let mut k = j;
            while k > 0 {
                meter.tick()?;
                let l = shares[k - 1];
                let r = shares[k];
                let left = (std::cmp::Reverse(l.2), input.bands[l.0].owner);
                let right = (std::cmp::Reverse(r.2), input.bands[r.0].owner);
                if left <= right {
                    break;
                }
                shares.swap(k - 1, k);
                k -= 1;
            }
        }
        let extra = usize::try_from(s.partial - allocated).map_err(|_| AnnualError::Overflow)?;
        if extra > shares.len() {
            return Err(AnnualError::Invalid("proportional remainder"));
        }
        for (k, &(j, whole, _)) in shares.iter().enumerate() {
            let amount = whole + u128::from(k < extra);
            let o = checked.owner[j];
            marginal[o] = add(marginal[o], amount)?;
        }
        balance.marginal_evaporation.0 = add(balance.marginal_evaporation.0, s.partial)?;
    }
    let mut lakes = Vec::new();
    let mut leaf_net = Vec::new();
    lakes
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    leaf_net
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    for (i, node) in input.nodes.iter().enumerate() {
        meter.tick()?;
        if lake_cells[i] > 0 {
            lakes.push(RepresentativeLake {
                basin: node.id,
                surface: HeightMm::new(state[i].height),
                submerged_cells: lake_cells[i],
                geometric_volume: Litres(lake_volume[i]),
                potential_spill: node.spill,
            });
        }
        if checked.children[i].is_empty() {
            let a = leaf_account[i];
            let lake = (lake_cells[a] > 0).then_some(input.nodes[a].id);
            let to_signed = |v| i128::try_from(v).map_err(|_| AnnualError::Overflow);
            leaf_net.push(LeafNet {
                leaf: node.id,
                account: if lake.is_some() {
                    input.nodes[a].id
                } else {
                    node.id
                },
                lake,
                surface: lake.map(|_| HeightMm::new(state[a].height)),
                runoff: node.local_runoff,
                paid_wet_cost: Litres(wet_cost[i]),
                marginal_cost: Litres(marginal[i]),
                net_litres: to_signed(node.local_runoff.0)?
                    .checked_sub(to_signed(wet_cost[i])?)
                    .and_then(|v| v.checked_sub(i128::try_from(marginal[i]).ok()?))
                    .ok_or(AnnualError::Overflow)?,
            });
        }
    }
    let inputs = add(balance.land_precipitation.0, balance.lake_precipitation.0)?;
    let losses = add(
        add(balance.land_loss.0, balance.lake_evaporation.0)?,
        balance.marginal_evaporation.0,
    )?;
    if inputs != add(losses, add(sea, exterior)?)? {
        return Err(AnnualError::Invalid("annual conservation"));
    }
    Ok(AnnualSolution {
        lakes,
        leaf_net,
        balance,
        events: meter.used,
    })
}
