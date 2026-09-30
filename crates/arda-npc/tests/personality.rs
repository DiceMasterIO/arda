//! Personalities are complete and fit the job.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::JobCategory;
use common::{content, everyone, port_town, town};

#[test]
fn ideals_fit_the_job() {
    let tables = content("personality.json");
    let excluded = |category: &str| -> Vec<String> {
        tables["exclusions"][category]
            .as_array()
            .map(|a| a.iter().map(|t| t.as_str().unwrap().to_string()).collect())
            .unwrap_or_default()
    };
    let mut clergy = 0;
    for (profile, buildings) in [town(), port_town()] {
        for npc in everyone(&profile, &buildings) {
            let tags = &npc.personality.ideal.tags;
            match npc.job.category {
                JobCategory::Religion => {
                    clergy += 1;
                    assert!(
                        !tags.contains(&"irreligious".to_string()),
                        "doubting {}",
                        npc.job.title
                    );
                    for t in excluded("religion") {
                        assert!(!tags.contains(&t));
                    }
                }
                JobCategory::Military => {
                    assert!(
                        !tags.contains(&"pacifism".to_string()),
                        "pacifist {}",
                        npc.job.title
                    );
                }
                _ => {}
            }
        }
    }
    assert!(clergy > 5);
}

#[test]
fn personalities_are_complete() {
    let (profile, buildings) = town();
    let tables = content("personality.json");
    let trait_axis = |text: &str| {
        tables["traits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["text"] == text)
            .map(|t| t["axis"].as_str().unwrap().to_string())
    };
    for npc in everyone(&profile, &buildings) {
        let p = &npc.personality;
        assert_eq!(p.traits.len(), 2);
        assert_ne!(
            trait_axis(&p.traits[0]),
            trait_axis(&p.traits[1]),
            "two traits on one axis"
        );
        assert!(!p.ideal.name.is_empty() && !p.ideal.alignment.is_empty());
        for text in [&p.bond, &p.flaw, &p.mannerism, &p.summary] {
            assert!(!text.is_empty());
            assert!(
                !text.contains('{') && !text.contains('}'),
                "unfilled template: {text}"
            );
        }
        assert!(p
            .summary
            .to_lowercase()
            .contains(&npc.job.title.to_lowercase()));
    }
}
