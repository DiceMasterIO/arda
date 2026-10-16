//! `arda-settle`: generate or render a world's settlements, roads and realms.

use clap::{Parser, Subcommand, ValueEnum};
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
        /// Page look: `political` (realm edge bands, dash-dot borders,
        /// ivory roads) or `classic` (realm tints, dashed borders, red and
        /// yellow roads).
        #[arg(long, value_enum, default_value_t = PageStyleArg::Political)]
        style: PageStyleArg,
    },
}

/// `--style` of `render`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PageStyleArg {
    /// Realm edge bands and dash-dot borders over ivory cased roads.
    Political,
    /// Realm tints and ribbons, dashed borders, red and yellow roads.
    Classic,
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
            style,
        } => arda_settle::render::render_with(
            &world,
            &out,
            quality,
            arda_settle::render::PageOptions {
                full: full_frame,
                style: match style {
                    PageStyleArg::Political => arda_settle::render::PageStyle::Political,
                    PageStyleArg::Classic => arda_settle::render::PageStyle::Classic,
                },
            },
        )
        .map(|()| {
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
