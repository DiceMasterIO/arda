//! Verify one completed fine world against its persisted source and saved beds.
//! Usage: cargo run -p arda --release --example verify_fine_world -- WORLD SOURCE

use arda::World;
use arda_core::{
    formats::{
        hydrology::{BasinNodeRow, FixedRecord, TableReader},
        overview::decode_overview,
    },
    hydrology::{AnnualCatchment, GlobalLake, GlobalReach, HydrologyMetadata, SharedCrossing},
    CellCoord, TerrainPoint,
};
use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

fn same_files(a: &Path, b: &Path) -> Result<u64, Box<dyn Error>> {
    let mut a = BufReader::new(File::open(a)?);
    let mut b = BufReader::new(File::open(b)?);
    let mut aa = [0_u8; 65_536];
    let mut bb = [0_u8; 65_536];
    let mut count = 0_u64;
    loop {
        let na = a.read(&mut aa)?;
        let nb = b.read(&mut bb)?;
        if na != nb || aa[..na] != bb[..nb] {
            return Err("fine source copy differs from input".into());
        }
        if na == 0 {
            return Ok(count);
        }
        count += u64::try_from(na)?;
    }
}

fn verify_table<T: FixedRecord>(
    root: &Path,
    name: &str,
    expected: u64,
) -> Result<(), Box<dyn Error>> {
    let path = root.join("hydrology").join(name);
    let max_bytes = std::fs::metadata(&path)?.len();
    let mut table = TableReader::<_, T>::open(File::open(&path)?, max_bytes, expected)?;
    if table.count() != expected {
        return Err(format!("{name} count differs from metadata").into());
    }
    let mut seen = 0_u64;
    while table.next_record()?.is_some() {
        seen += 1;
    }
    if seen != expected {
        return Err(format!("{name} stream ended early").into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = args.next().ok_or("missing world directory")?;
    let source = args.next().ok_or("missing source terrain file")?;
    if args.next().is_some() {
        return Err("usage: verify_fine_world WORLD SOURCE".into());
    }
    let root = Path::new(&root);
    let world = World::load(root)?;
    let descriptor = world
        .manifest()
        .fine_terrain
        .ok_or("fine descriptor absent")?;
    let copied_bytes = same_files(&root.join(arda_core::FINE_TERRAIN_PATH), Path::new(&source))?;
    let mut fine = world.fine_terrain(1 << 20)?.ok_or("fine reader absent")?;
    let mut exported_cells = 0_u64;
    let mut sea = 0_u64;
    let mut land = 0_u64;
    let mut lake = 0_u64;
    let mut watercourse = 0_u64;
    for (ax, ay) in world.area_coords() {
        let area = world.read_area(ax, ay)?;
        for y in 0..512_u16 {
            for x in 0..512_u16 {
                let cell = area
                    .cells()
                    .get(CellCoord::new(x, y).ok_or("cell coordinate")?);
                let global_x = i64::from(ax) * 512 + i64::from(x);
                let global_y = i64::from(ay) * 512 + i64::from(y);
                let sampled = fine
                    .sample(TerrainPoint {
                        x_um: global_x * 100_000_000,
                        y_um: global_y * 100_000_000,
                    })?
                    .ok_or("fine field misses exported cell")?;
                if sampled != cell.height {
                    return Err(format!(
                        "saved cell height differs from fine at ({global_x},{global_y}): {} vs {}",
                        cell.height.raw(),
                        sampled.raw()
                    )
                    .into());
                }
                exported_cells += 1;
                match cell.terrain {
                    arda::TerrainKind::Sea => sea += 1,
                    arda::TerrainKind::Land => land += 1,
                    arda::TerrainKind::Lake => lake += 1,
                }
                watercourse += u64::from(cell.watercourse_order > 0);
            }
        }
    }

    let overview_path = root.join("continent/overview.bin");
    let overview = decode_overview(
        &overview_path.display().to_string(),
        &std::fs::read(&overview_path)?,
    )?;
    let mut continent_nodes = 0_u64;
    let mut land_links = 0_u64;
    let mut root_nodes = 0_u64;
    let neighbors: [(i32, i32); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    for y in 0..overview.height {
        for x in 0..overview.width {
            let at = usize::try_from(y * overview.width + x)?;
            let cell = &overview.cells[at];
            let sampled = fine
                .sample(TerrainPoint {
                    x_um: i64::from(x) * 1_000_000_000,
                    y_um: i64::from(y) * 1_000_000_000,
                })?
                .ok_or("fine field misses continent cell")?;
            if sampled != cell.height {
                return Err(format!("continent height differs from fine at ({x},{y})").into());
            }
            continent_nodes += 1;
            if let Some(dir) = cell.downstream {
                let (dx, dy) = neighbors[usize::from(dir)];
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= overview.width || ny >= overview.height {
                    return Err(format!("continent receiver exits grid at ({x},{y})").into());
                }
                let to = &overview.cells[usize::try_from(ny * overview.width + nx)?];
                if cell.height.raw() > 0 && to.height.raw() > 0 {
                    if to.catchment_km2 < cell.catchment_km2
                        || to.discharge.raw() < cell.discharge.raw()
                    {
                        return Err(
                            format!("continent land-flow load decreases at ({x},{y})").into()
                        );
                    }
                    land_links += 1;
                }
            } else {
                root_nodes += 1;
            }
        }
    }
    // Every saved receiver chain must terminate; the overview exposes no
    // filled routing surface, so this checks topology without claiming that
    // raw bed heights decrease along every lake/spill link.
    let mut state = vec![0_u8; overview.cells.len()];
    for start in 0..overview.cells.len() {
        let mut path = Vec::new();
        let mut current = start;
        let mut ended_at_root = false;
        while state[current] == 0 {
            state[current] = 1;
            path.push(current);
            let Some(dir) = overview.cells[current].downstream else {
                ended_at_root = true;
                break;
            };
            let x = i32::try_from(current % usize::try_from(overview.width)?)?;
            let y = i32::try_from(current / usize::try_from(overview.width)?)?;
            let (dx, dy) = neighbors[usize::from(dir)];
            current = usize::try_from((y + dy) * overview.width + (x + dx))?;
        }
        if state[current] == 1 && !ended_at_root {
            return Err("continent receiver graph contains a cycle".into());
        }
        for at in path {
            state[at] = 2;
        }
    }

    let metadata_path = root.join("hydrology/metadata.bin");
    let mut table =
        TableReader::<_, HydrologyMetadata>::open(File::open(&metadata_path)?, 4096, 1)?;
    let metadata = table.read_at(0)?; // Decoding validates annual budget closure.
    let budget = metadata.budget;
    let sources = budget.land_precipitation.0 + budget.lake_precipitation.0;
    let sinks = budget.land_loss.0
        + budget.lake_evaporation.0
        + budget.marginal_evaporation.0
        + budget.sea_outflow.0
        + budget.domain_outflow.0;
    if sources != sinks {
        return Err("annual water ledger does not close".into());
    }
    verify_table::<BasinNodeRow>(root, "basins.bin", metadata.basin_count)?;
    verify_table::<GlobalLake>(root, "lakes.bin", metadata.lake_count)?;
    verify_table::<GlobalReach>(root, "reaches.bin", metadata.reach_count)?;
    verify_table::<SharedCrossing>(root, "crossings.bin", metadata.crossing_count)?;
    verify_table::<AnnualCatchment>(root, "catchments.bin", metadata.catchment_count)?;
    println!(
        "recipe_version={} attempt={}",
        descriptor.recipe_version, descriptor.attempt
    );
    println!("copied_fine_bytes={copied_bytes}");
    println!("exported_cells={exported_cells} sea={sea} land={land} lake={lake} watercourse={watercourse}");
    println!("continent_nodes={continent_nodes} land_receiver_links={land_links} routing_roots={root_nodes}");
    println!("hydrology_model_revision={} modeled={}x{} basins={} lakes={} reaches={} crossings={} catchments={}", metadata.model_revision, metadata.domain.width_cells, metadata.domain.height_cells, metadata.basin_count, metadata.lake_count, metadata.reach_count, metadata.crossing_count, metadata.catchment_count);
    println!(
        "annual_sources_l={sources} annual_sinks_l={sinks} sea_outflow_l={} domain_outflow_l={}",
        budget.sea_outflow.0, budget.domain_outflow.0
    );
    Ok(())
}
