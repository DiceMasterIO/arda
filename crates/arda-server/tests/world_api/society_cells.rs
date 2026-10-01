//! Adapter A11 (logic/16 §api-cell-society): with a `society/`, the cell
//! contract's `road`, `built_by`, `land_use` and `realm_id` are the
//! society rasters', in `/v1/cell` and in both area forms alike.

use crate::support::{get_with, society_dir, society_state};
use axum::http::StatusCode;
use serde_json::Value;

/// The area of the first owned cell, and global cells in it: the first
/// owned, the first on a road and the first wild one, in row-major order.
fn picks() -> ((usize, usize), Vec<(u32, u32)>) {
    let soc = society_dir().join("society");
    let (w, _, codes, owner) = arda_settle::output::read_landuse(&soc.join("landuse.bin")).unwrap();
    let first = owner.iter().position(|&o| o != 0).unwrap();
    let area = ((first % w) / 512, (first / w) / 512);
    let mut out = Vec::new();
    let roads = arda_settle::output::read_road_map(&soc.join("roads.bin")).ok();
    let mut want = [false; 3];
    for i in 0..owner.len() {
        let (x, y) = (i % w, i / w);
        if (x / 512, y / 512) != area {
            continue;
        }
        let road = roads.as_ref().is_some_and(|r| r.2[i] != 0);
        for (k, hit) in [owner[i] != 0, road, codes[i] == 0].into_iter().enumerate() {
            if hit && !want[k] {
                want[k] = true;
                out.push((u32::try_from(x).unwrap(), u32::try_from(y).unwrap()));
            }
        }
    }
    (area, out)
}

#[tokio::test]
async fn cell_society_fields_come_from_the_society_rasters() {
    let soc = society_dir().join("society");
    let (w, _, codes, owner) = arda_settle::output::read_landuse(&soc.join("landuse.bin")).unwrap();
    let (_, _, realms) = arda_settle::output::read_realm_map(&soc.join("realms.bin")).unwrap();
    let roads = arda_settle::output::read_road_map(&soc.join("roads.bin")).ok();
    let state = society_state();
    let ((ax, ay), cells) = picks();
    assert!(cells.len() >= 2, "an owned cell and a wild one");
    let area = get_with(state.clone(), &format!("/v1/area/{ax}/{ay}/cells"), None).await;
    assert_eq!(
        area.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&area.body)
    );
    let area = area.json();
    let column = |name: &str, i: usize| area["columns"][name][i].clone();
    for (gx, gy) in cells {
        let r = get_with(state.clone(), &format!("/v1/cell/{gx}/{gy}"), None).await;
        assert_eq!(r.status, StatusCode::OK);
        let c = r.json();
        let i = gy as usize * w + gx as usize;
        let id = |v: u64| {
            if v == 0 {
                Value::Null
            } else {
                Value::from(v.to_string())
            }
        };
        assert_eq!(c["built_by"], id(u64::from(owner[i])), "{gx},{gy}");
        assert_eq!(c["realm_id"], id(u64::from(realms[i])), "{gx},{gy}");
        let land_use = arda_ids::LandUse::from_code(codes[i]).unwrap();
        assert_eq!(c["land_use"], serde_json::to_value(land_use).unwrap());
        if let Some((_, _, r)) = &roads {
            let road = arda_ids::RoadClass::from_code(r[i]).unwrap();
            assert_eq!(c["road"], serde_json::to_value(road).unwrap());
        }
        // The columnar body agrees.
        let li = (gy as usize % 512) * 512 + gx as usize % 512;
        assert_eq!(column("built_by", li), Value::from(owner[i]));
        assert_eq!(column("realm_id", li), Value::from(realms[i]));
        assert_eq!(column("land_use", li), Value::from(codes[i]));
        let legend =
            &area["legend"]["road"][usize::try_from(column("road", li).as_u64().unwrap()).unwrap()];
        assert_eq!(*legend, c["road"]);
    }
}
