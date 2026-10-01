//! Admission of an annual input: hierarchy shape, band ordering, joins and
//! forcing bounds, checked before any filling begins.

use super::*;

pub(super) fn validate(
    input: &AnnualInput<'_>,
    limits: AnnualLimits,
    meter: &mut Meter,
) -> Result<Checked> {
    let n = input.nodes.len();
    let b = input.bands.len();
    let nn = u64::try_from(n).map_err(|_| AnnualError::Overflow)?;
    let bb = u64::try_from(b).map_err(|_| AnnualError::Overflow)?;
    if nn > limits.nodes || bb > limits.bands || required_ram(nn, bb)? > limits.ram_bytes {
        return Err(AnnualError::Limit("admission"));
    }
    if input.children.len() > n || input.joins.len() > n {
        return Err(AnnualError::Invalid("too many child/join entries"));
    }
    if input.nodes.windows(2).any(|w| w[0].id >= w[1].id) {
        return Err(AnnualError::Invalid("node identity order"));
    }
    let mut parent = vec_of(n, None)?;
    let mut children = vec_of(n, Vec::new())?;
    let mut adjacency = vec_of(n, Vec::new())?;
    let mut owner = vec_of(b, 0)?;
    let mut child_seen = vec_of(n, false)?;
    let mut child_slots = vec_of(input.children.len(), false)?;
    let mut band_seen = vec_of(b, false)?;
    let mut own_p = vec_of(n, 0_u128)?;
    let mut own_a = vec_of(n, 0_u128)?;
    let mut source_p = 0_u128;
    let mut source_a = 0_u128;
    let mut total_e = 0_u128;
    for (i, node) in input.nodes.iter().enumerate() {
        meter.tick()?;
        if node.children.start > node.children.end
            || node.children.end > input.children.len()
            || node.bands.start >= node.bands.end
            || node.bands.end > b
        {
            return Err(AnnualError::Invalid("node spans"));
        }
        let spill = node
            .spill
            .ok_or(AnnualError::Invalid("missing physical exit"))?;
        if node.floor.raw() > node.birth.raw() || node.birth.raw() >= spill.sill.raw() {
            return Err(AnnualError::Invalid("nonpositive retained capacity"));
        }
        if let Some(p) = node.parent {
            let p = index(input, p)?;
            if input.nodes[p].birth.raw() != spill.sill.raw() {
                return Err(AnnualError::Invalid("parent sill"));
            }
            parent[i] = Some(p);
        } else if node.destination.is_none() {
            return Err(AnnualError::Invalid("root destination"));
        }
        let p = node.local_precipitation.0;
        let a = node.local_land_loss.0;
        if a > p || node.local_runoff.0 != p - a {
            return Err(AnnualError::Invalid("R=P-A"));
        }
        if !node.children.is_empty() && (p != 0 || a != 0) {
            return Err(AnnualError::Invalid("internal source"));
        }
        if node.children.is_empty() && node.floor != node.birth {
            return Err(AnnualError::Invalid("leaf floor"));
        }
        source_p = add(source_p, p)?;
        source_a = add(source_a, a)?;
        for j in node.children.clone() {
            meter.tick()?;
            if child_slots[j] {
                return Err(AnnualError::Invalid("overlapping child span"));
            }
            child_slots[j] = true;
            let c = index(input, input.children[j])?;
            if child_seen[c] || input.nodes[c].parent != Some(node.id) {
                return Err(AnnualError::Invalid("child relation"));
            }
            child_seen[c] = true;
            children[i]
                .try_reserve(1)
                .map_err(|_| AnnualError::Limit("allocation"))?;
            children[i].push(c);
        }
        if !children[i].is_empty()
            && (children[i].len() < 2
                || children[i]
                    .iter()
                    .map(|&c| input.nodes[c].floor.raw())
                    .min()
                    != Some(node.floor.raw()))
        {
            return Err(AnnualError::Invalid("contracted join/floor"));
        }
        let mut previous = None;
        let mut group_owners = BTreeSet::new();
        for j in node.bands.clone() {
            meter.tick()?;
            let band = &input.bands[j];
            if band_seen[j] {
                return Err(AnnualError::Invalid("overlapping band span"));
            }
            band_seen[j] = true;
            if band.cells == 0
                || band.bed.raw() < node.birth.raw()
                || band.bed.raw() >= spill.sill.raw()
                || previous.is_some_and(|z| z > band.bed.raw())
            {
                return Err(AnnualError::Invalid("band geometry/order"));
            }
            if previous != Some(band.bed.raw()) {
                group_owners.clear();
            }
            if !group_owners.insert(band.owner) {
                return Err(AnnualError::Invalid("duplicate band owner/height"));
            }
            previous = Some(band.bed.raw());
            let o = index(input, band.owner)?;
            if !input.nodes[o].children.is_empty() {
                return Err(AnnualError::Invalid("band nonleaf owner"));
            }
            owner[j] = o;
            let p = band.precipitation.0;
            let a = band.effective_land_loss.0;
            let e = band.open_water_evaporation.0;
            if a > p || a > e {
                return Err(AnnualError::Invalid("band P/A/E"));
            }
            own_p[o] = add(own_p[o], p)?;
            own_a[o] = add(own_a[o], a)?;
            total_e = add(total_e, e)?;
        }
        if input.bands[node.bands.start].bed != node.birth {
            return Err(AnnualError::Invalid("missing birth band"));
        }
    }
    if source_p > MAX_ANNUAL || total_e > MAX_ANNUAL {
        return Err(AnnualError::Limit("annual forcing bound"));
    }
    if child_slots.contains(&false) || band_seen.contains(&false) {
        return Err(AnnualError::Invalid("unowned input row"));
    }
    for i in 0..n {
        meter.tick()?;
        if parent[i].is_some() != child_seen[i] {
            return Err(AnnualError::Invalid("unlisted child"));
        }
        if own_p[i] > input.nodes[i].local_precipitation.0
            || own_a[i] > input.nodes[i].local_land_loss.0
        {
            return Err(AnnualError::Invalid("band exceeds owner source"));
        }
        for j in input.nodes[i].bands.clone() {
            if !descendant(owner[j], i, &parent, meter)? {
                return Err(AnnualError::Invalid("band outside owner subtree"));
            }
        }
    }
    let mut union: Vec<usize> = vec_of(n, 0)?;
    for (i, u) in union.iter_mut().enumerate() {
        *u = i;
    }
    let mut join_count = vec_of(n, 0_usize)?;
    let mut witnesses = BTreeSet::new();
    for (j, edge) in input.joins.iter().enumerate() {
        meter.tick()?;
        if !witnesses.insert(edge.witness) {
            return Err(AnnualError::Invalid("duplicate witness"));
        }
        let p = index(input, edge.parent)?;
        let l = index(input, edge.left_child)?;
        let r = index(input, edge.right_child)?;
        let ll = index(input, edge.left_leaf)?;
        let rr = index(input, edge.right_leaf)?;
        if l == r
            || parent[l] != Some(p)
            || parent[r] != Some(p)
            || !children[ll].is_empty()
            || !children[rr].is_empty()
            || !descendant(ll, l, &parent, meter)?
            || !descendant(rr, r, &parent, meter)?
        {
            return Err(AnnualError::Invalid("join endpoint membership"));
        }
        let ul = find(l, &mut union, meter)?;
        let ur = find(r, &mut union, meter)?;
        if ul == ur {
            return Err(AnnualError::Invalid("join cycle"));
        }
        union[ur] = ul;
        join_count[p] += 1;
        adjacency[l]
            .try_reserve(1)
            .map_err(|_| AnnualError::Limit("allocation"))?;
        adjacency[r]
            .try_reserve(1)
            .map_err(|_| AnnualError::Limit("allocation"))?;
        adjacency[l].push((j, r));
        adjacency[r].push((j, l));
    }
    for i in 0..n {
        if join_count[i] != children[i].len().saturating_sub(1) {
            return Err(AnnualError::Invalid("incomplete join tree"));
        }
    }
    // Parent heights are strictly increasing, so parent chains cannot cycle.
    // Cross-root terminal routes must independently form an acyclic graph.
    let mut marks = vec_of(n, 0_u8)?;
    let mut chain = Vec::new();
    chain
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    for start in 0..n {
        if parent[start].is_some() || marks[start] == 2 {
            continue;
        }
        let mut at = start;
        loop {
            meter.tick()?;
            if marks[at] == 1 {
                return Err(AnnualError::Invalid("root destination cycle"));
            }
            if marks[at] == 2 {
                break;
            }
            marks[at] = 1;
            chain.push(at);
            match input.nodes[at].destination {
                Some(AnnualDestination::Basin(id)) => {
                    at = index(input, id)?;
                    if !children[at].is_empty() {
                        return Err(AnnualError::Invalid("nonleaf receiving terminal"));
                    }
                    while let Some(p) = parent[at] {
                        meter.tick()?;
                        at = p;
                    }
                }
                Some(AnnualDestination::Sea | AnnualDestination::DomainExport) => break,
                None => return Err(AnnualError::Invalid("missing root destination")),
            }
        }
        for i in chain.drain(..) {
            marks[i] = 2;
        }
    }
    Ok(Checked {
        parent,
        children,
        adjacency,
        owner,
        source_p,
        source_a,
    })
}
