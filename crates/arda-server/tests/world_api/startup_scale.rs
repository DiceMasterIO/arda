//! A default-size world opens with the default configuration in well under
//! a minute and is admitted (the 0.8.1+ startup hang: the town-plan warm-up
//! thrashed the refine source's area bound, then admission refused the
//! world). Needs a settled default-size world with its society: set
//! `ARDA_WORLD` (for example a seed-42 recipe-9 world after `arda settle`
//! and `arda society build`). Mind the memory: the open holds about 3 GB.

use crate::support::library_dir;
use arda_server::{AppState, ServerConfig};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const OPEN_BUDGET: Duration = Duration::from_secs(60);

#[test]
#[ignore = "needs ARDA_WORLD, a settled default-size world with society; slow"]
fn a_default_size_world_opens_with_the_default_config_within_a_minute() {
    let dir = PathBuf::from(std::env::var("ARDA_WORLD").expect("ARDA_WORLD"));
    let mut config = ServerConfig::new(dir);
    config.library = library_dir();
    let start = Instant::now();
    let state = AppState::open(&config).unwrap();
    let took = start.elapsed();
    let w = state.people.as_ref().expect("a settled world");
    println!(
        "opened {} settlements in {:.1} s; resident {:?}; plans cached {} bytes",
        w.files.settlements.settlements.len(),
        took.as_secs_f64(),
        state.resident,
        w.cached_plan_bytes().unwrap()
    );
    assert!(took < OPEN_BUDGET, "open took {took:?}");
}
