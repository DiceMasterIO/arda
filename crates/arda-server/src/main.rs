//! `arda-server --world <dir> --port 8787 [--library <dir>]`: serves one
//! stored world and a tactical asset library under `/v1`.

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
    /// Decoded-area cache budget, MiB.
    #[arg(long, default_value_t = 1024)]
    area_cache_mib: usize,
    /// Fine-terrain window cache budget, MiB.
    #[arg(long, default_value_t = 512)]
    fine_cache_mib: usize,
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
    /// Most connections served at once; further clients wait.
    #[arg(long, default_value_t = 64)]
    max_connections: usize,
    /// Background threads warming prefetched tactical cells (0 disables).
    #[arg(long, default_value_t = arda_server::tactical::prefetch::DEFAULT_WORKERS)]
    prefetch_workers: usize,
    /// Sheet mapping file reshaping every served NPC into the game's schema
    /// (goal 69); checked at startup, a bad file refuses to start.
    #[arg(long)]
    sheet_mapping: Option<PathBuf>,
}

async fn run(args: Args) -> Result<(), String> {
    let world = args.world.ok_or("--world is required")?;
    let mut config = ServerConfig::new(world);
    config.query.area_bytes = args.area_cache_mib << 20;
    config.query.fine_bytes = args.fine_cache_mib << 20;
    config.overview.max_quality_px = args.max_overview_px;
    config.overview.tile_base_px = args.tile_base_px;
    config.library = args.library;
    config.serve = ServeLimits {
        header_read_timeout: Duration::from_secs(args.header_timeout_s),
        max_connections: args.max_connections,
    };
    config.prefetch_workers = args.prefetch_workers;
    config.sheet_mapping = args.sheet_mapping;
    let limits = config.serve;
    let state = tokio::task::spawn_blocking(move || AppState::open(&config))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let addr = SocketAddr::new(args.host, args.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("binding {addr}: {e}"))?;
    println!(
        "arda-server {} serving seed {} on http://{addr}/v1 (tactical library {}, sheet mapping {})",
        env!("CARGO_PKG_VERSION"),
        state.query.world().seed(),
        state.tactical.library_version(),
        state.sheet_mapping.as_ref().map_or("none", |m| m.name())
    );
    serve(listener, router(Arc::new(state)), limits, shutdown_signal())
        .await
        .map_err(|e| e.to_string())
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
