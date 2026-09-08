//! Saved representative annual hydrology; large IDs and litres use decimal strings.
use arda_core::hydrology::*;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AccountOut {
    Reach { global_reach_id: String },
    Lake { global_basin_id: String },
    Junction { global_junction_id: String },
    Sea,
    DomainExport,
}

fn account(v: ReceivingAccount) -> AccountOut {
    match v {
        ReceivingAccount::Reach(id) => AccountOut::Reach {
            global_reach_id: id.0.to_string(),
        },
        ReceivingAccount::Lake(id) => AccountOut::Lake {
            global_basin_id: id.0.to_string(),
        },
        ReceivingAccount::Junction(id) => AccountOut::Junction {
            global_junction_id: id.0.to_string(),
        },
        ReceivingAccount::Sea => AccountOut::Sea,
        ReceivingAccount::DomainExport => AccountOut::DomainExport,
    }
}

#[derive(Serialize)]
struct SpillOut {
    from_global_cell: [u32; 2],
    to_global_cell: Option<[u32; 2]>,
    sill_mm: i32,
    receiving: AccountOut,
}

fn spill(v: SpillConnection) -> SpillOut {
    SpillOut {
        from_global_cell: [v.from.x, v.from.y],
        to_global_cell: v.to.map(|c| [c.x, c.y]),
        sill_mm: v.sill.raw(),
        receiving: account(v.receiving),
    }
}

#[derive(Serialize)]
struct LakeOut {
    global_basin_id: String,
    surface_mm: i32,
    deepest_bed_mm: i32,
    submerged_cells: u32,
    outlet: Option<SpillOut>,
    annual_outflow_litres: String,
    mean_outflow_milli_cumecs: u64,
}
fn lake(v: &GlobalLake) -> LakeOut {
    LakeOut {
        global_basin_id: v.basin.0.to_string(),
        surface_mm: v.surface.raw(),
        deepest_bed_mm: v.deepest_bed.raw(),
        submerged_cells: v.submerged_cells,
        outlet: v.outlet.map(spill),
        annual_outflow_litres: v.annual_outflow.0.to_string(),
        mean_outflow_milli_cumecs: v.mean_outflow.raw(),
    }
}

#[derive(Serialize)]
struct CatchmentOut {
    catchment_id: String,
    terminal_global_cell: [u32; 2],
    contributing_cells: u32,
    physical_basin_id: Option<String>,
    representative_lake_id: Option<String>,
    potential_spill: Option<SpillOut>,
    receiving: AccountOut,
}
fn catchment(v: &AnnualCatchment) -> CatchmentOut {
    CatchmentOut {
        catchment_id: v.catchment.0.to_string(),
        terminal_global_cell: [v.terminal.x, v.terminal.y],
        contributing_cells: v.contributing_cells,
        physical_basin_id: v.basin.map(|id| id.0.to_string()),
        representative_lake_id: v.representative_lake.map(|id| id.0.to_string()),
        potential_spill: v.potential_spill.map(spill),
        receiving: account(v.receiving),
    }
}

#[derive(Serialize)]
struct ReachOut {
    global_reach_id: String,
    is_point: bool,
    from_global_cell: [u32; 2],
    to_global_cell: [u32; 2],
    receiving: AccountOut,
    catchment_id: String,
    drainage_cells: u32,
    annual_volume_litres: String,
    mean_discharge_milli_cumecs: u64,
}
fn reach(v: &GlobalReach) -> ReachOut {
    ReachOut {
        global_reach_id: v.id.0.to_string(),
        is_point: v.id.is_point(),
        from_global_cell: [v.from.x, v.from.y],
        to_global_cell: [v.to.x, v.to.y],
        receiving: account(v.receiving),
        catchment_id: v.catchment.0.to_string(),
        drainage_cells: v.drainage_cells,
        annual_volume_litres: v.annual_volume.0.to_string(),
        mean_discharge_milli_cumecs: v.mean_discharge.raw(),
    }
}

#[derive(Serialize)]
struct CrossingOut {
    id: [[u32; 2]; 2],
    from_global_cell: [u32; 2],
    to_global_cell: [u32; 2],
    global_reach_id: String,
    catchment_id: String,
    drainage_cells: u32,
    annual_volume_litres: String,
    mean_discharge_milli_cumecs: u64,
    receiving: AccountOut,
}
fn crossing(v: &SharedCrossing) -> CrossingOut {
    CrossingOut {
        id: [[v.id.low.x, v.id.low.y], [v.id.high.x, v.id.high.y]],
        from_global_cell: [v.from.x, v.from.y],
        to_global_cell: [v.to.x, v.to.y],
        global_reach_id: v.reach.0.to_string(),
        catchment_id: v.catchment.0.to_string(),
        drainage_cells: v.drainage_cells,
        annual_volume_litres: v.annual_volume.0.to_string(),
        mean_discharge_milli_cumecs: v.mean_discharge.raw(),
        receiving: account(v.receiving),
    }
}

#[derive(Serialize)]
pub(crate) struct HydrologyOut {
    model_revision: u32,
    representative: &'static str,
    flow_semantics: &'static str,
    annual_seconds: u32,
    lake_membership: &'static str,
    lakes: Vec<LakeOut>,
    reaches: Vec<ReachOut>,
    catchments: Vec<CatchmentOut>,
    crossings: Vec<CrossingOut>,
}
pub(crate) fn hydrology(v: &AreaHydrologyContext) -> HydrologyOut {
    HydrologyOut {
        model_revision: v.model_revision,
        representative: "representative_annual_balance",
        flow_semantics: "mean_annual_discharge",
        annual_seconds: 31_536_000,
        lake_membership: "physical_bed_mm < surface_mm",
        lakes: v.lakes.iter().map(lake).collect(),
        reaches: v.reaches.iter().map(reach).collect(),
        catchments: v.catchments.iter().map(catchment).collect(),
        crossings: v.crossings.iter().map(crossing).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{DischargeMilli, GlobalCell, HeightMm};
    #[test]
    fn large_ids_and_whole_annual_litres_survive_without_float_rounding() {
        let id = (1u64 << 63) + 7;
        let whole = u128::from(u64::MAX) + 123;
        let context = AreaHydrologyContext {
            reaches: vec![GlobalReach {
                id: ReachId(id),
                from: GlobalCell { x: 15, y: 15 },
                to: GlobalCell { x: 16, y: 15 },
                receiving: ReceivingAccount::DomainExport,
                catchment: CatchmentId(17),
                drainage_cells: 20,
                annual_volume: Litres(whole),
                mean_discharge: DischargeMilli::new(u64::try_from(whole / 31_536_000).unwrap()),
            }],
            ..AreaHydrologyContext::default()
        };
        let bytes = serde_json::to_string(&hydrology(&context)).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&bytes).unwrap();
        let reach = &parsed["reaches"][0];
        assert_eq!(
            reach["global_reach_id"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            id
        );
        assert_eq!(
            reach["annual_volume_litres"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
            whole
        );
        assert_eq!(bytes, serde_json::to_string(&hydrology(&context)).unwrap());
    }
    #[test]
    fn annual_surface_and_tiny_supported_outflow_have_no_temporal_claims() {
        let context = AreaHydrologyContext {
            lakes: vec![GlobalLake {
                basin: BasinId(1),
                surface: HeightMm::new(1000),
                deepest_bed: HeightMm::new(999),
                submerged_cells: 1,
                outlet: Some(SpillConnection {
                    from: GlobalCell { x: 0, y: 0 },
                    to: None,
                    sill: HeightMm::new(1000),
                    receiving: ReceivingAccount::DomainExport,
                }),
                annual_outflow: Litres(1),
                mean_outflow: DischargeMilli::new(0),
            }],
            ..AreaHydrologyContext::default()
        };
        let parsed = serde_json::to_value(hydrology(&context)).unwrap();
        assert_eq!(parsed["representative"], "representative_annual_balance");
        assert_eq!(parsed["model_revision"], 2);
        assert_eq!(parsed["lakes"][0]["surface_mm"], 1000);
        assert_eq!(parsed["lakes"][0]["annual_outflow_litres"], "1");
        assert_eq!(parsed["lakes"][0]["mean_outflow_milli_cumecs"], 0);
        let bytes = serde_json::to_string(&parsed).unwrap();
        for obsolete in [
            "september",
            "january",
            "periodic",
            "active_outflow",
            "q48",
            "catchment_states",
            "shoreline_exchanges",
            "basin_diagnostics",
        ] {
            assert!(!bytes.contains(obsolete), "obsolete key {obsolete}");
        }
    }
    #[test]
    fn junction_reference_is_a_decimal_identity_without_an_invented_table() {
        let id = JunctionId::at(arda_core::GlobalCell { x: 13, y: 40000 });
        let parsed = serde_json::to_value(account(ReceivingAccount::Junction(id))).unwrap();
        assert_eq!(parsed["kind"], "junction");
        assert_eq!(parsed["global_junction_id"], id.0.to_string());
        assert!(parsed.get("global_reach_id").is_none());
        let context = serde_json::to_value(hydrology(&AreaHydrologyContext::default())).unwrap();
        assert!(context.get("junctions").is_none());
    }
    #[test]
    fn physical_point_export_has_no_invented_step_or_separate_table() {
        let at = GlobalCell { x: 0, y: 13 };
        let g = GlobalReach {
            id: ReachId::point(at).unwrap(),
            from: at,
            to: at,
            receiving: ReceivingAccount::DomainExport,
            catchment: CatchmentId(5),
            drainage_cells: 3,
            annual_volume: Litres(1),
            mean_discharge: DischargeMilli::new(0),
        };
        let context = AreaHydrologyContext {
            reaches: vec![g.clone()],
            ..AreaHydrologyContext::default()
        };
        let value = serde_json::to_value(hydrology(&context)).unwrap();
        let r = &value["reaches"][0];
        assert_eq!(r["is_point"], true);
        assert_eq!(r["global_reach_id"], g.id.0.to_string());
        assert_eq!(r["from_global_cell"], r["to_global_cell"]);
        assert_eq!(r["annual_volume_litres"], "1");
        assert_eq!(r["mean_discharge_milli_cumecs"], 0);
        assert_eq!(r["receiving"]["kind"], "domain_export");
        assert!(value.get("points").is_none());
    }
}
