//! Plain-text rendering of an NPC and their sheet, for GM tools and logs.

use std::fmt::Write as _;

use crate::npc::Npc;
use crate::rules::Ability;
use crate::sheet::{SheetKind, Spellcasting};

fn signed(v: i8) -> String {
    if v >= 0 {
        format!("+{v}")
    } else {
        v.to_string()
    }
}

/// One roster line: name, ancestry, age, job and sheet summary.
#[must_use]
pub fn roster_line(npc: &Npc) -> String {
    let sheet = match &npc.sheet.kind {
        SheetKind::StatBlock { name, .. } => name.clone(),
        SheetKind::Class { class, level, .. } => format!("{class} {level}"),
    };
    format!(
        "{:<34} {:<10} {:>3}  {:<24} {}",
        npc.name.full(),
        npc.ancestry_name,
        npc.age,
        npc.job.title,
        sheet
    )
}

fn spell_lines(out: &mut String, casting: &Spellcasting) {
    let slots: Vec<String> = casting
        .slots
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .map(|(i, n)| format!("L{}×{n}", i + 1))
        .collect();
    let _ = writeln!(
        out,
        "Spellcasting: {} (save DC {}, {} to hit){}; slots {}",
        casting.ability.abbr(),
        casting.save_dc,
        signed(casting.attack_bonus),
        if casting.pact_magic {
            ", Pact Magic"
        } else {
            ""
        },
        slots.join(" ")
    );
    let max = casting.spells.iter().map(|s| s.level).max().unwrap_or(0);
    for level in 0..=max {
        let names: Vec<&str> = casting
            .spells
            .iter()
            .filter(|s| s.level == level)
            .map(|s| s.name.as_str())
            .collect();
        if !names.is_empty() {
            let label = if level == 0 {
                "Cantrips".to_string()
            } else {
                format!("Level {level}")
            };
            let _ = writeln!(out, "  {label}: {}", names.join(", "));
        }
    }
}

/// A full, readable sheet with personality.
#[must_use]
pub fn sheet_text(npc: &Npc) -> String {
    let s = &npc.sheet;
    let mut out = String::new();
    let _ = writeln!(out, "== {} ({:?}) ==", npc.name.full(), npc.id);
    let race = npc
        .subrace
        .clone()
        .unwrap_or_else(|| npc.ancestry_name.clone());
    let _ = writeln!(
        out,
        "{race}, {} {:?}, age {}; {} ({:?}, {:?})",
        s.size, npc.sex, npc.age, npc.job.title, npc.social_rank, npc.lifestyle
    );
    let what = match &s.kind {
        SheetKind::StatBlock {
            name,
            challenge_rating,
            xp,
        } => format!("SRD stat block: {name} (CR {challenge_rating}, {xp} XP)"),
        SheetKind::Class {
            class,
            subclass,
            level,
            background,
        } => format!(
            "{class} {level}{}; background {} ({})",
            subclass
                .as_ref()
                .map(|s| format!(" ({s})"))
                .unwrap_or_default(),
            background.name,
            background.text
        ),
    };
    let _ = writeln!(out, "{what}");
    let _ = writeln!(
        out,
        "AC {} ({}), HP {} ({}), Speed {} ft., Proficiency {}",
        s.armor_class,
        if s.armor.is_empty() {
            "no armour".to_string()
        } else {
            s.armor.join(", ")
        },
        s.hit_points,
        s.hit_dice,
        s.speed,
        signed(s.proficiency_bonus)
    );
    let abilities: Vec<String> = Ability::ALL
        .iter()
        .map(|&a| {
            format!(
                "{} {} ({})",
                a.abbr(),
                s.abilities[a.index()],
                signed(s.modifiers[a.index()])
            )
        })
        .collect();
    let _ = writeln!(out, "{}", abilities.join("  "));
    let saves: Vec<String> = s
        .saves
        .iter()
        .filter(|v| v.proficient)
        .map(|v| format!("{} {}", v.ability.abbr(), signed(v.bonus)))
        .collect();
    if !saves.is_empty() {
        let _ = writeln!(out, "Saves: {}", saves.join(", "));
    }
    let skills: Vec<String> = s
        .skills
        .iter()
        .map(|k| {
            format!(
                "{} {}{}",
                k.name,
                signed(k.bonus),
                if k.expertise { "*" } else { "" }
            )
        })
        .collect();
    if !skills.is_empty() {
        let _ = writeln!(out, "Skills: {}", skills.join(", "));
    }
    let mut senses = s.senses.clone();
    senses.push(format!("passive Perception {}", s.passive_perception));
    let _ = writeln!(
        out,
        "Senses: {}; Languages: {}",
        senses.join(", "),
        s.languages.join(", ")
    );
    if !s.damage_resistances.is_empty() {
        let _ = writeln!(out, "Resistances: {}", s.damage_resistances.join(", "));
    }
    for a in &s.attacks {
        let extra = if a.extra.is_empty() {
            String::new()
        } else {
            format!(" {}", a.extra.join(", "))
        };
        let _ = writeln!(
            out,
            "  {} ({:?}, {}): {} to hit, {} ({}) {}{extra}",
            a.name,
            a.kind,
            a.range,
            signed(a.to_hit),
            a.average,
            a.damage,
            a.damage_type
        );
    }
    let features = if s.features.is_empty() {
        "none".to_string()
    } else {
        s.features.join(", ")
    };
    let _ = writeln!(out, "Features: {features}");
    if let Some(casting) = &s.spellcasting {
        spell_lines(&mut out, casting);
    }
    let items: Vec<String> = s
        .equipment
        .iter()
        .map(|i| {
            if i.quantity > 1 {
                format!("{} ×{}", i.name, i.quantity)
            } else {
                i.name.clone()
            }
        })
        .collect();
    let _ = writeln!(
        out,
        "Equipment: {}; purse {} gp {} sp {} cp",
        items.join(", "),
        s.coins.gp,
        s.coins.sp,
        s.coins.cp
    );
    let p = &npc.personality;
    let _ = writeln!(out, "Personality: {}", p.summary);
    let _ = writeln!(out, "  Traits: {}", p.traits.join(" "));
    let _ = writeln!(
        out,
        "  Ideal ({}; {}): {}",
        p.ideal.name, p.ideal.alignment, p.ideal.text
    );
    let _ = writeln!(out, "  Bond: {}", p.bond);
    let _ = writeln!(out, "  Flaw: {}", p.flaw);
    let _ = writeln!(out, "  Mannerism: {}", p.mannerism);
    let ties: Vec<String> = npc
        .relationships
        .iter()
        .map(|r| format!("{:?} #{}", r.kind, r.other))
        .collect();
    let _ = writeln!(out, "Relationships: {}", ties.join(", "));
    out
}
