//! The player's own light: a white glow above the head that the original
//! turns on only while the player wears a helm that sees in the dark, and
//! puts out while a hand holds a torch. Rules: `docs/PLAYER.md` (The player
//! light).
//!
//! **The switch** is read whenever the player's possessions or hands
//! change (the original reads it at the start of play, after a respawn and
//! on every change of equipment): on when any worn item's body has special
//! kind 1, then off when either hand of the record's out set holds an item
//! with a held light's section (and in Exarust when the body carries its
//! torch). Turned on, the light is made; turned off, it is removed.
//!
//! **The light** is an omni light, colour (1, 1, 1), radius 1200 cm,
//! multiplier 2, softness 4, falling off with distance, made at 245 cm
//! with the intensity 0.01 and rising to 0.4: each frame, while it is
//! under 0.4 less the single-precision epsilon, it gains 0.0005 per
//! millisecond of the frame (in double precision, then single), clamped
//! to 0.4. Each frame it is placed 80 cm above the player's head; when
//! that is more than 1 cm above the head, a 10 cm ball swept up from the
//! head through the level (not the props) puts it where the ball first
//! meets the ceiling. Not while the game is paused.
//!
//! The second off rule is an item in either hand whose weapon section is of
//! kind 3 while the player's held part carries its lit flag (one part's
//! flag, read for either hand). Exarust's bodies do not keep that flag yet,
//! so the running switch passes it off; the rule is here and tested. Not in Exarust: the Mind power that forces the light on, and the
//! spectating view's own light.
//!
//! Evidence: the controls' light maker, frame and switch and the
//! equipment's switch, read in the instructions
//! (`context/re/player-control-hand.md`, The player light).

use bevy::prelude::*;
use exa_character::held::Hands;
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::Possessions;
use exa_physics::Physics;
use exa_physics::body::Ragdoll;

use crate::player::PlayerBody;
use crate::torch::CarriesTorch;

/// Centimetres per metre.
const CM: f32 = 100.0;
/// The light's target intensity, and the intensity it is made with.
pub const TARGET: f32 = 0.4;
pub const START: f32 = 0.01;
/// The intensity gained per millisecond of a frame.
pub const RAMP_PER_MS: f64 = 0.0005;
/// The single-precision epsilon of the original's compare.
const EPSILON: f32 = 1.192_09e-7;
/// The light's radius (cm), multiplier and softness.
pub const RADIUS_CM: f32 = 1200.0;
pub const MULTIPLIER: f32 = 2.0;
pub const SOFTNESS: f32 = 4.0;
/// Where the light is made (cm up) before its first frame.
const MADE_AT_CM: f32 = 245.0;
/// The light hangs this far (cm) above the head.
pub const ABOVE_HEAD_CM: f32 = 80.0;
/// The ceiling probe: only past this gap (cm), with a ball of this radius
/// (cm).
const PROBE_FROM_CM: f32 = 1.0;
const PROBE_RADIUS_CM: f32 = 10.0;
/// The head's bone.
const HEAD: usize = 10;

/// Whether a held item's weapon section puts the light out: kind 3, while
/// the player's held part is flagged lit.
pub fn weapon_puts_out(weapon_kind: Option<u8>, held_part_lit: bool) -> bool {
    weapon_kind == Some(3) && held_part_lit
}

/// Whether the player light burns: a worn item that sees in the dark, put
/// out by a light in either hand or by a lit held weapon.
pub fn switched_on(
    worn_sees: impl IntoIterator<Item = bool>,
    hands_light: [bool; 2],
    weapon_lit: bool,
) -> bool {
    let mut on = false;
    for sees in worn_sees {
        if sees {
            on = true;
        }
    }
    if hands_light.iter().any(|&l| l) || weapon_lit {
        on = false;
    }
    on
}

/// The intensity after a frame of `ms` milliseconds.
pub fn ramp(intensity: f32, ms: f32) -> f32 {
    let under = intensity < TARGET - EPSILON;
    if !under {
        return intensity;
    }
    let up = (f64::from(ms) * RAMP_PER_MS + f64::from(intensity)) as f32;
    if up > TARGET { TARGET } else { up }
}

/// The light's height (cm): 80 cm over the head at `head` (cm), or where a
/// ball swept up from the head meets the ceiling, `sweep(gap)` giving how
/// far it went before it did.
pub fn height(head: f32, sweep: impl FnOnce(f32) -> Option<f32>) -> f32 {
    let y = head + ABOVE_HEAD_CM;
    let gap = y - head;
    if gap > PROBE_FROM_CM
        && let Some(t) = sweep(gap)
    {
        return head + t;
    }
    y
}

/// The player light's node and its intensity.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PlayerLight {
    pub intensity: f32,
}

/// The light record the player light is made from (metres).
pub fn light_record() -> exa_formats::rfc::Light {
    exa_formats::rfc::Light {
        flags: 0,
        color: [1.0, 1.0, 1.0],
        multiplier: MULTIPLIER,
        intensity: START,
        radius: RADIUS_CM / CM,
        softness: SOFTNESS,
        spot: None,
        ranged: true,
        animation: None,
    }
}

pub struct PlayerLightPlugin;

impl Plugin for PlayerLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (switch_player_light, place_player_light).chain());
    }
}

/// Makes or removes the light when what the player wears or holds changes.
#[allow(clippy::type_complexity)]
fn switch_player_light(
    mut commands: Commands,
    catalogue: Option<Res<ItemCatalogue>>,
    players: Query<
        (&Possessions, Option<&Hands>, Has<CarriesTorch>),
        (
            With<PlayerBody>,
            Or<(Changed<Possessions>, Changed<Hands>, Added<PlayerBody>)>,
        ),
    >,
    lights: Query<Entity, With<PlayerLight>>,
) {
    let Some((possessions, hands, torch)) = players.iter().next() else {
        return;
    };
    let kind = |id: u32| catalogue.as_ref().and_then(|c| c.kind(id));
    let worn = possessions
        .worn
        .iter()
        .map(|&id| kind(id).is_some_and(|k| k.sight));
    let held = |i: usize| {
        let id = possessions.out[i].id;
        id != 0 && kind(id).is_some_and(|k| k.light)
    };
    let mut hands_light = [held(0), held(1)];
    // Exarust's carried torch is outside the record: its hand holds a light.
    if torch && hands.is_some_and(|h| h.main.is_some_and(|m| m.light) || h.off_light) {
        hands_light[0] = true;
    }
    // The player's held part keeps no lit flag yet, so the weapon's second
    // off rule is passed off here.
    let on = switched_on(worn, hands_light, false);
    let made = lights.iter().next();
    match (on, made) {
        (true, None) => {
            info!("player light: on");
            commands.spawn((
                Name::new("PlayerLight"),
                PlayerLight { intensity: START },
                exa_world::light_bundle(&light_record()),
                Transform::from_xyz(0.0, MADE_AT_CM / CM, 0.0),
            ));
        }
        (false, Some(light)) => {
            info!("player light: off");
            commands.entity(light).despawn();
        }
        _ => {}
    }
}

/// The light's frame: its ramp, then its place over the head.
fn place_player_light(
    time: Res<Time<Virtual>>,
    physics: Option<Res<Physics>>,
    players: Query<&Ragdoll, With<PlayerBody>>,
    mut lights: Query<(&mut PlayerLight, &mut PointLight, &mut Transform)>,
) {
    if time.is_paused() {
        return;
    }
    let Ok((mut light, mut point, mut at)) = lights.single_mut() else {
        return;
    };
    let ms = time.delta_secs() * 1000.0;
    let intensity = ramp(light.intensity, ms);
    if intensity != light.intensity {
        light.intensity = intensity;
        point.intensity = intensity * exa_world::LUMENS_PER_INTENSITY;
    }
    let Some(head) = players
        .iter()
        .next()
        .and_then(|b| b.part_of(HEAD))
        .map(|p| Vec3::from(p.transform().translation))
    else {
        return;
    };
    let y = height(head.y * CM, |gap| {
        physics.as_ref().and_then(|p| {
            p.level_sweep(head, Vec3::Y, gap / CM, PROBE_RADIUS_CM / CM)
                .map(|t| t * CM)
        })
    });
    let place = Vec3::new(head.x, y / CM, head.z);
    if at.translation != place {
        at.translation = place;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A night-vision helm turns it on, a light in either hand puts it out,
    /// and nothing worn leaves it off.
    #[test]
    fn the_switch() {
        assert!(switched_on([false, true, false], [false, false], false));
        assert!(!switched_on([false, true], [false, true], false));
        assert!(!switched_on([true], [true, false], false));
        assert!(!switched_on([], [false, false], false));
        // A weapon of kind 3 with a lit held part puts it out too.
        assert!(weapon_puts_out(Some(3), true));
        assert!(!weapon_puts_out(Some(3), false));
        assert!(!weapon_puts_out(Some(2), true));
        assert!(!weapon_puts_out(None, true));
        assert!(!switched_on([true], [false, false], true));
    }

    /// From 0.01 it gains 0.0005 per millisecond, so a 16 ms frame adds
    /// 0.008; it stops at 0.4, which it reaches in 780 ms.
    #[test]
    fn the_light_rises_to_point_four() {
        let first = ramp(START, 16.0);
        assert!((first - 0.018).abs() < 1e-6, "{first}");
        assert_eq!(ramp(0.399, 16.0), TARGET);
        assert_eq!(ramp(TARGET, 16.0), TARGET);
        let mut i = START;
        let mut ms = 0;
        while i < TARGET {
            i = ramp(i, 1.0);
            ms += 1;
        }
        assert!((779..=781).contains(&ms), "{ms} ms");
    }

    /// 80 cm above the head, or under a ceiling a ball meets first.
    #[test]
    fn the_light_hangs_over_the_head_under_the_ceiling() {
        assert_eq!(height(150.0, |_| None), 230.0);
        assert_eq!(height(150.0, |gap| (gap == 80.0).then_some(35.0)), 185.0);
    }

    #[test]
    fn the_light_record() {
        let l = light_record();
        assert_eq!(l.color, [1.0, 1.0, 1.0]);
        assert_eq!((l.radius, l.multiplier, l.softness), (12.0, 2.0, 4.0));
        assert_eq!(l.intensity, 0.01);
        assert!(l.ranged);
    }
}
