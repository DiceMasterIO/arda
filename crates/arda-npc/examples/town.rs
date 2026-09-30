//! Builds a sample market town of about 1,500 people, prints a roster and
//! three full sheets, and writes `out/npc/town.json`.
//!
//! Run with `cargo run -p arda-npc --example town`.

use std::error::Error;
use std::path::PathBuf;

use arda_npc::input::{label, BuildingFunction};
use arda_npc::sheet::SheetKind;
use arda_npc::text::{roster_line, sheet_text};
use arda_npc::{sample, Generator, JobCategory};

fn main() -> Result<(), Box<dyn Error>> {
    let (world_seed, town, buildings) = sample::market_town();
    let generator = Generator::new(world_seed, &town, &buildings)?;
    let population = generator.population()?;

    println!("{} — market town, seed {world_seed:#x}", town.name);
    println!(
        "{} people in {} households and {} buildings; {} notables stored, {} commoners on demand",
        population.population,
        population.households.len(),
        buildings.len(),
        population.npcs.len(),
        population.commoner_count
    );
    println!("\nOccupations by category:");
    for (category, count) in population.category_counts() {
        println!("  {:<12} {count:>5}", format!("{category:?}"));
    }
    println!("\nMost common jobs:");
    let mut counts = population.job_counts();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (job, count) in counts.iter().take(12) {
        println!("  {job:<18} {count:>5}");
    }

    println!("\nNotables:");
    for npc in population.notables() {
        let place = npc
            .workplace_building
            .and_then(|b| buildings.iter().find(|x| x.id == b))
            .map_or_else(
                || "-".to_string(),
                |b| format!("{} #{}", label(b.function, b.craft_tag()), b.id),
            );
        println!("  {}  @ {place}", roster_line(npc));
    }

    println!("\nA few commoners:");
    for id in generator.ids().step_by(97).take(12) {
        let npc = generator.npc(id)?;
        if !npc.notable {
            println!("  {}", roster_line(&npc));
        }
    }

    let notables = population.notables();
    let priest = notables
        .iter()
        .find(|n| n.job.category == JobCategory::Religion && n.sheet.level() > 0);
    let leader = notables
        .iter()
        .filter(|n| n.job.category == JobCategory::Military)
        .max_by_key(|n| n.sheet.level());
    let guard = generator
        .ids()
        .map(|id| generator.npc(id))
        .find(|n| {
            n.as_ref().is_ok_and(
                |n| matches!(&n.sheet.kind, SheetKind::StatBlock { name, .. } if name == "Guard"),
            )
        })
        .transpose()?;
    for npc in [priest.cloned(), leader.cloned(), guard]
        .into_iter()
        .flatten()
    {
        println!("\n{}", sheet_text(&npc));
    }

    let dock = buildings
        .iter()
        .find(|b| b.function == BuildingFunction::Dock)
        .map(|b| b.id);
    if let Some(dock) = dock {
        println!(
            "Workers at dock #{}: {}",
            dock.0,
            population.workers(dock).len()
        );
    }

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out/npc");
    std::fs::create_dir_all(&dir)?;
    let path = dir.canonicalize()?.join("town.json");
    std::fs::write(&path, serde_json::to_string_pretty(&population)?)?;
    println!("wrote {}", path.display());
    Ok(())
}
