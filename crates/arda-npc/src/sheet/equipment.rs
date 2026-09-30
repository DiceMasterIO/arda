//! Equipment and coins by job and wealth.

use crate::data::content::JobData;
use crate::npc::Lifestyle;
use crate::rng::Rng;
use crate::sheet::{Coins, Item};

/// Wealth band used to choose armour quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wealth {
    Poor,
    Modest,
    Rich,
}

impl Wealth {
    pub(crate) fn from_lifestyle(lifestyle: Lifestyle) -> Self {
        match lifestyle {
            Lifestyle::Wretched | Lifestyle::Squalid | Lifestyle::Poor => Self::Poor,
            Lifestyle::Modest | Lifestyle::Comfortable => Self::Modest,
            Lifestyle::Wealthy | Lifestyle::Aristocratic => Self::Rich,
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Poor => 0,
            Self::Modest => 1,
            Self::Rich => 2,
        }
    }
}

/// Adds `name` to `items`, merging quantities of the same item.
pub(crate) fn add_item(items: &mut Vec<Item>, name: &str, quantity: u16) {
    match items.iter_mut().find(|i| i.name == name) {
        Some(item) => item.quantity = item.quantity.saturating_add(quantity),
        None => items.push(Item {
            name: name.to_string(),
            quantity,
        }),
    }
}

/// Clothes, tools and gear of a job at a lifestyle.
pub(crate) fn job_items(
    job: &JobData,
    craft_tool: Option<&str>,
    lifestyle: Lifestyle,
) -> Vec<Item> {
    let mut items = Vec::new();
    let clothes = match lifestyle {
        Lifestyle::Wealthy | Lifestyle::Aristocratic => "Clothes, fine",
        _ => "Clothes, common",
    };
    add_item(&mut items, clothes, 1);
    if lifestyle >= Lifestyle::Poor {
        add_item(&mut items, "Pouch", 1);
    }
    for name in job
        .tools
        .iter()
        .chain(&job.gear)
        .map(String::as_str)
        .chain(craft_tool)
    {
        if name != clothes {
            add_item(&mut items, name, 1);
        }
    }
    items
}

/// Coins in the purse for a lifestyle.
pub(crate) fn coins(lifestyle: Lifestyle, rng: &mut Rng) -> Coins {
    match lifestyle {
        Lifestyle::Wretched => Coins {
            gp: 0,
            sp: 0,
            cp: rng.range(0, 8),
        },
        Lifestyle::Squalid => Coins {
            gp: 0,
            sp: rng.range(0, 2),
            cp: rng.range(2, 20),
        },
        Lifestyle::Poor => Coins {
            gp: 0,
            sp: rng.range(1, 8),
            cp: rng.range(0, 30),
        },
        Lifestyle::Modest => Coins {
            gp: rng.range(1, 5),
            sp: rng.range(0, 15),
            cp: rng.range(0, 30),
        },
        Lifestyle::Comfortable => Coins {
            gp: rng.range(5, 20),
            sp: rng.range(0, 20),
            cp: 0,
        },
        Lifestyle::Wealthy => Coins {
            gp: rng.range(20, 80),
            sp: rng.range(0, 30),
            cp: 0,
        },
        Lifestyle::Aristocratic => Coins {
            gp: rng.range(80, 300),
            sp: 0,
            cp: 0,
        },
    }
}
