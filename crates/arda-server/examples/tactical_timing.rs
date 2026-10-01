//! Times cold tactical images stage by stage (goal 50):
//! `cargo run --release -p arda-server --example tactical_timing -- <world> <gx> <gy> [ppsq]`.
//!
//! Opens the world like the server (which also draws the town and city
//! plans), then times, for cell `(gx, gy)`: the composed apron block
//! (refinement, ways, fields and town overlays) and the render plus PNG
//! encode; then a quarter-block window across the corner of four cells, and
//! the east neighbour. `PLAN_ID=<settlement>` first times that settlement's
//! plan and interior salts on their own. The server logs the render and
//! encode stages to stderr.

use arda_server::tactical::block::BlockRequest;
use arda_server::{AppState, ServerConfig};
use std::path::PathBuf;
use std::time::Instant;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let world = PathBuf::from(args.get(1).ok_or("usage: <world> <gx> <gy> [ppsq]")?);
    let num = |i: usize, d: u32| {
        args.get(i)
            .map_or(Ok(d), |t| t.parse::<u32>().map_err(|e| e.to_string()))
    };
    let (gx, gy, ppsq) = (num(2, 0)?, num(3, 0)?, num(4, 64)?);
    let t = Instant::now();
    let state = AppState::open(&ServerConfig::new(world)).map_err(|e| e.to_string())?;
    eprintln!("open (with town plans) {:.1} ms", ms(t));
    let (w, h) = state.query.cells();
    let extent = (i64::from(w) * 64, i64::from(h) * 64);
    let tac = &state.tactical;
    if let (Some(people), Ok(id)) = (&state.people, std::env::var("PLAN_ID")) {
        let id: u64 = id
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let t = Instant::now();
        let plan = people.plan(id).map_err(|e| e.to_string())?;
        eprintln!("plan {id}: {:.1} ms, some={}", ms(t), plan.is_some());
        if let Some(p) = plan.as_ref() {
            let t = Instant::now();
            let salt = p.buildings.first().map(|b| p.interior_salt(b));
            eprintln!("interior salts: {:.1} ms ({salt:?})", ms(t));
        }
    }
    let quarter = BlockRequest {
        gsx0: i64::from(gx) * 64 + 48,
        gsy0: i64::from(gy) * 64 + 48,
        w: 32,
        h: 32,
        demo_at: None,
    };
    for (label, req) in [
        ("cell", BlockRequest::cell(gx, gy)),
        ("quarter", quarter),
        ("neighbour", BlockRequest::cell(gx + 1, gy)),
    ] {
        let t = Instant::now();
        let (block, _) = tac
            .world_render(&req, extent, ppsq, false)
            .map_err(|e| e.to_string())?;
        let compose = ms(t);
        let t = Instant::now();
        let (png, _) = tac
            .world_png(&req, extent, ppsq, false)
            .map_err(|e| e.to_string())?;
        eprintln!(
            "{label}: compose {compose:.1} ms ({}x{} sq), render+encode {:.1} ms, {} bytes",
            block.layout.width,
            block.layout.height,
            ms(t),
            png.bytes.len()
        );
    }
    Ok(())
}
