//! `exa-tool characters`: every character record of a level (with where it
//! places them) or of the arena's actors, read whole.

use anyhow::{Context, Result, anyhow};
use exa_formats::character_stats::{CharacterStats, Spawn};
use exa_formats::characters::{
    CharacterRecord, LevelCharacters, Placement, SAVED_HEAL_POOL, SAVED_POTION, SKILL_NAMES,
    technique_name,
};
use exa_formats::races::{self, Race};
use exa_rpk::{Archive, GameData, PackKind};

/// `source` is a level of `Resource.rpk` (`exanima01` or `exanima01.rfc`)
/// or `actors` for `actors.rpk`.
pub fn characters(game: &GameData, source: &str) -> Result<()> {
    let resource = game
        .pack(PackKind::Resource)
        .ok_or_else(|| anyhow!("Resource.rpk is not installed"))?;
    let races = resource
        .get("races.rdb")
        .map(races::parse)
        .transpose()?
        .unwrap_or_default();
    if source == "actors" {
        let actors = Archive::parse(
            resource
                .get("actors.rpk")
                .context("actors.rpk not found in Resource")?,
        )?;
        println!("actors.rpk: {} actors", actors.entries().len());
        for e in actors.entries() {
            match CharacterRecord::parse(0, actors.bytes(e)) {
                Ok(r) => print_record(&e.name, &r, &races, &[], false),
                Err(err) => println!("{}: not a record ({err})", e.name),
            }
        }
        return Ok(());
    }
    let entry = if source.ends_with(".rfc") {
        source.to_string()
    } else {
        format!("{source}.rfc")
    };
    let bytes = resource
        .get(&entry)
        .ok_or_else(|| anyhow!("{entry:?} not found in Resource"))?;
    let section = exa_formats::chunk::rfc_chunks(bytes)?
        .flatten()
        .find(|c| c.tag == exa_formats::characters::CHARACTERS)
        .ok_or_else(|| anyhow!("{entry} places no characters"))?;
    let level = LevelCharacters::parse(section.data)?;
    let story = entry.to_ascii_lowercase().starts_with("exanima");
    let entries = LevelCharacters::read_database(section.data)?;
    let unread = entries.iter().filter(|(_, r)| r.is_err()).count();
    println!(
        "{entry}: {} records ({unread} entries not records), {} placed",
        level.records.len(),
        level.placements.len()
    );
    for (id, r) in &entries {
        if let Ok(r) = r {
            let placed: Vec<&Placement> = level
                .placements
                .iter()
                .filter(|p| p.record() == *id)
                .collect();
            print_record(&format!("{id:#x}"), r, &races, &placed, story);
        }
    }
    Ok(())
}

/// `story`: the record is placed in a story level (else the arena).
fn print_record(
    label: &str,
    r: &CharacterRecord,
    races: &[Race],
    placed: &[&Placement],
    story: bool,
) {
    let race = races::of_character(races, r.race).map_or("?", |race| race.name.as_str());
    println!(
        "{label:<10} {:<16} {:<10} v{} {race} {}  feral {}  height {}  age {}  physique {:?}",
        r.name,
        r.surname,
        r.version,
        if r.female { "female" } else { "male" },
        r.feral,
        r.height,
        r.age,
        r.physique
    );
    let [yellow, red, focus] = r.pools.shares();
    println!(
        "    pools {yellow:.3} / {red:.3} / {focus:.3}  combat skill {}  experience {}  traits {:?}  voice {} ({}) pitch {} undead {}",
        r.combat_skill,
        r.experience,
        r.traits,
        r.voice,
        r.voice_index(),
        r.voice_pitch,
        r.undead_voice
    );
    for (slot, skill) in r.skills.iter().enumerate() {
        if skill.points == 0 {
            continue;
        }
        let learnt: Vec<String> = skill
            .learnt()
            .iter()
            .map(|&t| technique_name(slot, t).map_or(format!("#{t}"), str::to_string))
            .collect();
        let learnt = if learnt.is_empty() {
            "no technique yet".to_string()
        } else {
            learnt.join(", ")
        };
        println!("    {} {}: {learnt}", SKILL_NAMES[slot], skill.points);
    }
    let dead = placed.first().is_some_and(|p| p.is_dead());
    let s = CharacterStats::derive(r, races::of_character(races, r.race), Spawn { story, dead });
    println!(
        "    spawns with scale {:.4}  size {:.4}  pelvis {:.4} m  leg reach {:.4} m  most energy {:.4}  step reach {:.4} m  pins {:.4}  head gain {:.4} factor {:.4}  walking arms {:.4}  sprint {:.1}/s",
        s.scale,
        s.size,
        s.standing_height,
        s.leg_reach,
        s.max_energy,
        s.step_reach(0.0),
        s.pin_factor,
        s.head_gain,
        s.head_factor,
        s.walking_arms,
        s.sprint.thrust
    );
    println!(
        "    health {:.4} / {:.4} of {:.4}  rules {:#x}  perception {:.3}  voice pitch {:.3} volume {:.3} attack step {}",
        s.yellow,
        s.red,
        s.max_health,
        s.rules.0,
        s.perception,
        s.voice_pitch,
        s.voice_volume,
        s.attack_voice_step
    );
    let perks: Vec<&str> = r.perks().iter().map(|p| p.name()).collect();
    println!(
        "    perks [{}]  manoeuvring tier {}  deflection {}",
        perks.join(", "),
        r.manoeuvring_tier(),
        r.deflection()
    );
    let roles: Vec<String> = r
        .roles
        .iter()
        .map(|role| format!("{}{}", role.id, if role.explicit { "*" } else { "" }))
        .collect();
    let saved: Vec<String> = r
        .saved
        .iter()
        .map(|s| match s.tag {
            SAVED_HEAL_POOL => format!("heal pool {}", f32::from_bits(s.value)),
            SAVED_POTION => format!("potion until {}", s.value),
            tag => format!("{tag:#x} {}", s.value),
        })
        .collect();
    println!(
        "    roles [{}]  worn {}  inventory {}  relations {}  saved [{}]  powers {}",
        roles.join(" "),
        r.worn.len(),
        r.inventory.len(),
        r.relations,
        saved.join(", "),
        r.powers.as_ref().map_or(0, |p| p.known.len())
    );
    for p in placed {
        println!(
            "    placed at ({:.2}, {:.2}, {:.2}) m  yaw {:.2}  condition {:#x}",
            p.position[0], p.position[1], p.position[2], p.yaw, p.condition
        );
    }
}
