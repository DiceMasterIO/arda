//! `arda-server --world <dir> --port 8787 [--library <dir>]`: serves one
//! stored world and a tactical asset library under `/v1`.

use arda_server::ceiling::Pinned;
use arda_server::serve::{serve, ServeLimits};
use arda_server::{router, AppState, ServerConfig};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

/// Serve a generated arda world over HTTP.
#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    /// World directory produced by `arda generate`.
    #[arg(long, required_unless_present = "probe")]
    world: Option<PathBuf>,
    /// Health probe for container healthchecks: GET /v1/health on loopback
    /// at `--port`, exit 0 on 200, 1 otherwise. Serves nothing.
    #[arg(long)]
    probe: bool,
    /// TCP port.
    #[arg(long, default_value_t = 8787)]
    port: u16,
    /// Bind address; loopback by default.
    #[arg(long, default_value = "127.0.0.1")]
    host: std::net::IpAddr,
    /// Per-process memory ceiling, MiB (also `ARDA_MEMORY_MIB`): the worst
    /// case the server admits stays under it, and every budget not set by
    /// a flag below derives from it. Without it the ceiling is 16384 MiB
    /// and the budgets are their defaults.
    #[arg(long)]
    memory_mib: Option<u64>,
    /// Decoded-area cache budget, MiB [default: 256, or from --memory-mib].
    #[arg(long)]
    area_cache_mib: Option<usize>,
    /// Fine-terrain window cache budget, MiB [default: 128, or from
    /// --memory-mib].
    #[arg(long)]
    fine_cache_mib: Option<usize>,
    /// Largest `/overview.png` long edge clients may request, pixels.
    #[arg(long, default_value_t = 8192)]
    max_overview_px: u32,
    /// Tile pyramid base edge (256 × a power of two), pixels.
    #[arg(long, default_value_t = 4096)]
    tile_base_px: u32,
    /// Tactical asset library directory, or a `top:…:bottom` stack whose
    /// lower libraries fill in what the upper ones lack; validated at startup.
    #[arg(long, default_value = arda_server::tactical::DEFAULT_LIBRARY)]
    library: PathBuf,
    /// Seconds a client may take to send a complete request head.
    #[arg(long, default_value_t = 10)]
    header_timeout_s: u64,
    /// Most connections served at once; further clients wait [default: 32,
    /// or from --memory-mib].
    #[arg(long)]
    max_connections: Option<usize>,
    /// Background threads warming prefetched tactical cells (0 disables).
    #[arg(long, default_value_t = arda_server::tactical::prefetch::DEFAULT_WORKERS)]
    prefetch_workers: usize,
    /// Background threads baking posted cells (`POST /v1/tactical/bake`):
    /// 0 disables baking, 1 (the default and most) bakes one at a time.
    #[arg(long, default_value_t = arda_server::tactical::bake::DEFAULT_WORKERS)]
    bake_workers: usize,
    /// Budget for finished bake tile sets awaiting download, MiB; the
    /// oldest finished set is evicted when a new one does not fit
    /// [default: 384, or from --memory-mib].
    #[arg(long)]
    bake_result_mib: Option<usize>,
    /// Sheet mapping file reshaping every served NPC into the game's schema
    /// (goal 69); checked at startup, a bad file refuses to start.
    #[arg(long)]
    sheet_mapping: Option<PathBuf>,
}

async fn run(args: Args) -> Result<(), String> {
    let world = args.world.ok_or("--world is required")?;
    let mut config = ServerConfig::new(world);
    let memory_mib = match args.memory_mib {
        Some(m) => Some(m),
        None => memory_mib_env()?,
    };
    config.memory_ceiling = memory_mib.map(|m| m.saturating_mul(1 << 20));
    config.pinned = Pinned {
        area_cache: args.area_cache_mib.is_some(),
        fine_cache: args.fine_cache_mib.is_some(),
        max_connections: args.max_connections.is_some(),
        bake_result: args.bake_result_mib.is_some(),
    };
    if let Some(mib) = args.area_cache_mib {
        config.query.area_bytes = mib << 20;
    }
    if let Some(mib) = args.fine_cache_mib {
        config.query.fine_bytes = mib << 20;
    }
    config.overview.max_quality_px = args.max_overview_px;
    config.overview.tile_base_px = args.tile_base_px;
    config.library = args.library;
    config.serve = ServeLimits {
        header_read_timeout: Duration::from_secs(args.header_timeout_s),
        max_connections: args.max_connections.unwrap_or(config.serve.max_connections),
    };
    config.prefetch_workers = args.prefetch_workers;
    config.bake_workers = args.bake_workers;
    if let Some(mib) = args.bake_result_mib {
        config.bake_result_mib = mib;
    }
    config.sheet_mapping = args.sheet_mapping;
    let state = tokio::task::spawn_blocking(move || AppState::open(&config))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let addr = SocketAddr::new(args.host, args.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("binding {addr}: {e}"))?;
    println!(
        "arda-server {} serving seed {} on http://{addr}/v1 (tactical library {}, sheet mapping {}, \
         admitted {} MiB of {} MiB)",
        env!("CARGO_PKG_VERSION"),
        state.query.world().seed(),
        state.tactical.library_version(),
        state.sheet_mapping.as_ref().map_or("none", |m| m.name()),
        (state.admitted + state.resident.total()) >> 20,
        state.ceiling >> 20
    );
    let limits = state.serve_limits;
    serve(listener, router(Arc::new(state)), limits, shutdown_signal())
        .await
        .map_err(|e| e.to_string())
}

/// `ARDA_MEMORY_MIB`, when set: the ceiling for an image started without
/// arguments.
fn memory_mib_env() -> Result<Option<u64>, String> {
    match std::env::var("ARDA_MEMORY_MIB") {
        Ok(v) if !v.trim().is_empty() => v
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("ARDA_MEMORY_MIB {v:?} is not a whole number of MiB")),
        _ => Ok(None),
    }
}

/// Resolves on Ctrl-C (SIGINT) or, on Unix, SIGTERM, which is what
/// `docker stop` sends to PID 1.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Runs the `--probe` check against loopback.
fn run_probe(port: u16) -> ExitCode {
    let addr = SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), port);
    match arda_server::probe::probe(addr, Duration::from_secs(3)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("arda-server: {e}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    if args.probe {
        return run_probe(args.port);
    }
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("arda-server: {e}");
            ExitCode::FAILURE
        }
    }
}
