//! A village stands on its own cell (logic/09; the release gate's village
//! block): the town plan is centred on the settlement's site, so the cell
//! the world map marks as the village holds its people, not a fringe of
//! its houses while the green and the notables stand hundreds of metres
//! away around a bridge outside the village.

use crate::support::{get_with, society_state};
use arda_settle::model::Tier;
use axum::http::StatusCode;

/// Every village and hamlet plan has its market (the green, the village
/// centre) within its built core's radius of the settlement's site,
/// unless the planner put it on a harbour shore or under a castle: a
/// bridge or a road junction outside the village no longer drags the
/// whole plan off the cell the world map marks as the village.
#[test]
fn every_village_market_stands_at_its_site() {
    use arda_town::plan::types::FocalKind;
    let state = society_state();
    let people = state.people.as_deref().unwrap();
    let (mut planned, mut away) = (0, Vec::new());
    for s in &people.files.settlements.settlements {
        if !matches!(s.tier, Tier::Village | Tier::Hamlet) {
            continue;
        }
        let id = s.id.get();
        let plan = people.plan(id).unwrap();
        let Some(plan) = plan.as_ref() else { continue };
        planned += 1;
        if matches!(plan.focal.kind, FocalKind::Harbour | FocalKind::Castle) {
            continue;
        }
        let (_, record) = people.files.settlement(id).unwrap();
        let site = arda_people::town::site(record).unwrap();
        let core = arda_town::plan::params::Params::for_site(&site, 1.0).r_core;
        let off = plan.focal.market.dist(site.position());
        if off > core {
            away.push(format!(
                "{id} {:?} {:?}: market {off:.0} m from the site, core {core:.0} m",
                s.tier, plan.focal.kind
            ));
        }
    }
    assert!(planned > 20, "{planned} planned villages and hamlets");
    assert!(away.is_empty(), "{} of {planned}: {away:#?}", away.len());
}

/// The release gate's pick (the most populous riverine village, ties by
/// id): its own cell's scene by night shows its people at home.
#[tokio::test]
async fn the_largest_river_village_cell_shows_its_people_by_night() {
    let state = society_state();
    let people = state.people.as_deref().unwrap();
    let s = people
        .files
        .settlements
        .settlements
        .iter()
        .filter(|s| s.tier == Tier::Village && s.riverine)
        .min_by_key(|s| (std::cmp::Reverse(s.population), s.id.get()))
        .unwrap();
    let uri = format!(
        "/v1/tactical/cell/{}/{}/scene?time=night",
        s.cell_x, s.cell_y
    );
    let r = get_with(state.clone(), &uri, None).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    let scene = r.json();
    let id = s.id.get().to_string();
    let tokens = scene["tokens"].as_array().unwrap();
    let own = tokens
        .iter()
        .filter(|t| t["settlement_id"] == id.as_str())
        .count();
    assert!(own > 0, "{uri}: no token of village {id}: {tokens:?}");
    assert!(tokens.iter().all(|t| t["kind"] == "resident"), "{tokens:?}");
}
