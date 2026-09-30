//! `arda-settle`: generate or render a world's settlements, roads and realms.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    version,
    about = "Settlements, roads, realms and names for an arda world"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Derive `<world>/society/` from a generated world.
    Generate {
        /// The world directory.
        #[arg(long)]
        world: PathBuf,
    },
    /// Draw settlements, roads, realm borders and labels over the Atlas overview.
    Render {
        /// The world directory (with `society/` already generated).
        #[arg(long)]
        world: PathBuf,
        /// PNG to write.
        #[arg(long)]
        out: PathBuf,
        /// Overview long edge in pixels (512–32768).
        #[arg(long, default_value_t = 2048)]
        quality: u32,
        /// Keep the whole overview instead of cropping the page to the land.
        #[arg(long)]
        full_frame: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Generate { world } => {
            arda_settle::generate(&world, arda_settle::grid::MEMORY_BUDGET).and_then(
                |(society, dir)| {
                    let json = serde_json::to_string_pretty(&society.stats).map_err(|source| {
                        arda_settle::SettleError::Json {
                            path: "stats".into(),
                            source,
                        }
                    })?;
                    println!("wrote {}\n{json}", dir.display());
                    Ok(())
                },
            )
        }
        Command::Render {
            world,
            out,
            quality,
            full_frame,
        } => arda_settle::render::render(&world, &out, quality, full_frame).map(|()| {
            println!("wrote {}", out.display());
        }),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("arda-settle: {e}");
            ExitCode::FAILURE
        }
    }
}
