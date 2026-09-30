//! `arda settle` and `arda society build`: the settlement and society
//! stages over a generated world (goals 34–41 and 51–57), so the whole
//! pipeline runs from the CLI after `arda generate`:
//!
//! ```sh
//! arda generate --seed 42 --micro --terrain fine --out out/micro42
//! arda settle --world out/micro42
//! arda society build --world out/micro42
//! ```

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum SocietyCommand {
    /// Plan every settlement's town, simulate realm politics, economy and
    /// history, and store the notables: writes `<world>/society/society.json`
    /// and `notables.json` (run `arda settle` first).
    Build {
        /// World directory.
        #[arg(long)]
        world: PathBuf,
    },
}

/// `arda settle --world <dir>`.
pub(crate) fn settle(world: &std::path::Path) -> Result<()> {
    let (society, dir) = arda_settle::generate(world, arda_settle::grid::MEMORY_BUDGET)
        .with_context(|| format!("settling {}", world.display()))?;
    println!(
        "wrote {}: {} settlements, {} roads, {} realms",
        dir.display(),
        society.settlements.len(),
        society.network.roads.len(),
        society.realms.realms.len()
    );
    Ok(())
}

pub(crate) fn run(command: SocietyCommand) -> Result<()> {
    match command {
        SocietyCommand::Build { world } => {
            let r = arda_people::build(&world)
                .with_context(|| format!("building the society of {}", world.display()))?;
            println!(
                "wrote {}/society: {} settlements ({} with town plans), {} inhabitants, {} stored notables",
                world.display(),
                r.settlements,
                r.planned,
                r.population,
                r.notables
            );
            Ok(())
        }
    }
}
