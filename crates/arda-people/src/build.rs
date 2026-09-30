//! `arda society build`: plans every settlement, simulates `arda-society`
//! with the plan buildings as its explicit buildings (A14), and stores the
//! notables of every settlement (goal 56: only notables are stored).

use crate::files::{
    self, NotablesFile, StoredNotable, NOTABLES_FILE, NOTABLES_FORMAT, SOCIETY_FILE,
};
use crate::people::{slot, BuildingSource};
use crate::shared::SharedSource;
use crate::{town, PeopleError};
use arda_npc::Generator;
use arda_refine::Source;
use arda_society::WorldSettlements;
use std::path::Path;

/// What a build produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildReport {
    /// Settlements.
    pub settlements: usize,
    /// Settlements with a town plan.
    pub planned: usize,
    /// Stored notables.
    pub notables: usize,
    /// Inhabitants (stored and regenerated).
    pub population: u64,
}

/// Runs society and the notables over `<world>/society/` and writes
/// `society.json` and `notables.json` there.
///
/// # Errors
/// World, I/O, format, society and NPC failures.
pub fn build(world_dir: &Path) -> Result<BuildReport, PeopleError> {
    let src = SharedSource::open(world_dir)?;
    let dir = world_dir.join("society");
    let files = crate::files::SocietyFiles::read(&dir)?;
    let seed = src.seed();
    let mut report = BuildReport {
        settlements: files.settlements.settlements.len(),
        ..BuildReport::default()
    };
    let mut plans = Vec::with_capacity(report.settlements);
    let mut explicit = Vec::new();
    for plan in plan_all(&src, &files)? {
        if let Some((society, _)) = &plan {
            explicit.extend(society.iter().cloned());
            report.planned += 1;
        }
        plans.push(plan.map(|(_, npc)| npc));
    }
    let mut world = WorldSettlements::read_dir(&dir)?;
    world.buildings = explicit;
    let society = arda_society::simulate_society(seed, &world)?;
    arda_society::output::write_json(&society, &dir.join(SOCIETY_FILE))?;
    let mut stored = Vec::with_capacity(report.settlements);
    for ((s, record), (plan, soc)) in files
        .settlements
        .settlements
        .iter()
        .zip(&files.records)
        .zip(plans.into_iter().zip(&society.settlements))
    {
        let (buildings, source) = match plan {
            Some(b) => (b, BuildingSource::Plan),
            None => (town::mix_specs(s, &soc.buildings), BuildingSource::Mix),
        };
        let profile: arda_npc::SettlementProfile = serde_json::from_value(record.clone())
            .map_err(|e| PeopleError::format(&dir.join("settlements.json"), e))?;
        let slots: Vec<_> = soc.roles.iter().map(slot).collect();
        let pop = Generator::with_notables(seed, &profile, &buildings, &slots)?.population()?;
        report.notables += pop.npcs.len();
        report.population += u64::from(pop.population);
        stored.push(StoredNotable {
            settlement_id: s.id,
            buildings: source.key().to_string(),
            npcs: pop.npcs,
        });
    }
    files::write(
        &dir.join(NOTABLES_FILE),
        &NotablesFile {
            format_version: NOTABLES_FORMAT,
            seed: seed.to_string(),
            settlements: stored,
        },
    )?;
    Ok(report)
}

type Planned = Option<(
    Vec<arda_society::input::BuildingSpec>,
    Vec<arda_npc::BuildingSpec>,
)>;

/// Plans every settlement on parallel threads; each plan is a pure
/// function of its settlement and the world, and results are kept in
/// settlement order, so scheduling never changes the output.
fn plan_all(
    src: &SharedSource,
    files: &crate::files::SocietyFiles,
) -> Result<Vec<Planned>, PeopleError> {
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    let jobs: Vec<_> = files
        .settlements
        .settlements
        .iter()
        .zip(&files.records)
        .collect();
    let mut out = Vec::with_capacity(jobs.len());
    for chunk in jobs.chunks(threads.max(1) * 4) {
        let results: Vec<Result<Planned, PeopleError>> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&(s, record)| {
                    scope.spawn(move || {
                        let plan = town::plan(src, s, record, &files.roads.roads)?;
                        Ok(plan.map(|p| {
                            (
                                town::society_buildings(&p),
                                town::plan_specs(&p, s.population),
                            )
                        }))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().unwrap_or_else(|_| {
                        Err(PeopleError::World("a planner thread panicked".into()))
                    })
                })
                .collect()
        });
        for r in results {
            out.push(r?);
        }
    }
    Ok(out)
}
