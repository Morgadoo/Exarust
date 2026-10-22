//! The force torrent in the running game: the effect entity a torrent cast
//! makes, its streams fed from the caster's right hand while the cast is
//! held.
//!
//! - **Creator**: the power's cost a second, the caster and the power id
//!   0x10203.
//! - **Start** ([`start_torrent`]): cast motion 0x26 with that power id;
//!   when the motion starts the caster's cast hook becomes the torrent's
//!   ([`TORRENT_HOOK`]), which answers −1 and fizzles a charging torrent or
//!   ends a running one ([`Torrent::release`]).
//! - **The hook** ([`hook_called`]): a later cast's start calls it, and so
//!   does the body tick when the release flag is set, which also ends the
//!   cast. Running does not clear the hook: the torrent pours until the
//!   caster lets go or runs out.
//! - **Each tick** ([`tick_torrent`]): the torrent pays its drain while it
//!   charges and half of it while it runs; a shortfall calls the hook and
//!   ends the cast. Its spawn delays draw from the effects' shared roll.
//!   Once running (flags 0xb_0000), its live points push the limb spheres
//!   of other bodies they reach ([`strike_streams`]), and each push costs a
//!   struck character [`energy_after`]'s energy.
//! - **Leaving**: the frame that would build no triangles removes it
//!   ([`Torrent::builds_nothing`]): fizzled, or every stream spent.
//!
//! [inf: the frame's turned aim is the blast aim here (the player
//! behaviour's turn spring is not wired), and the contact is Exarust's
//! sphere test against a point's reach (its width times the limb scale,
//! plus the margin, plus 10), as for the blast.] Not wired: the character
//! knock, the simulated objects' points, the sounds, the light and the
//! draw.
//!
//! Evidence: the torrent's creator, start, hook, tick and frame, read from
//! `Exanima.exe`.

use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::Locomotor;

use super::barricade::WAKE_TICKS;
use super::blast_live::{blast_view, body_mass};
use super::cast::PowerCaster;
use super::casting::{self, CastBody, spend_tick};
use super::live::{EffectDraws, EffectRoll};
use crate::effects::blast::blast_aim;
use crate::effects::torrent::{
    POINTS, RUNNING_FLAGS, TORRENT_MOTION, Torrent, TorrentEvents, TorrentView, energy_after,
    limb_impulse, limb_terms, torrent_reach,
};
use crate::player::CursorGround;

/// Centimetres a metre.
const CM: f32 = 100.0;
const CHEST: usize = 9;
/// The torrent's power id.
pub const TORRENT: u32 = 0x10203;
/// What the torrent's hook answers.
pub const TORRENT_HOOK: i32 = -1;

/// A torrent in the scene.
#[derive(Component)]
pub struct LiveTorrent {
    pub torrent: Torrent,
    pub caster: Entity,
    /// The body the AI aims at; the player aims at the cursor.
    pub target: Option<Entity>,
    /// The caster's cast hook is this torrent's.
    pub hooked: bool,
}

/// The torrent's start on its caster: cast motion 0x26 with the power id,
/// and the hook when the motion starts.
pub fn start_torrent(body: &mut CastBody) -> bool {
    let started = casting::start_cast(body, TORRENT_MOTION, TORRENT);
    if started {
        body.cast_hook = Some(TORRENT_HOOK);
    }
    started
}

/// The view a torrent reads from its caster: the blast's, turned along
/// its aim.
pub fn torrent_view(caster: &Ragdoll, facing: Vec3, target: Option<Vec3>) -> Option<TorrentView> {
    let view = blast_view(caster, facing, target)?;
    Some(TorrentView {
        target: view.target,
        hand_node: view.hand_node,
        hand: view.hand,
        facing,
        turned_aim: blast_aim(view.hand_node, view.target, facing),
    })
}

/// A torrent for `caster` costing `cost` a second.
#[allow(clippy::too_many_arguments)]
pub fn new_torrent(
    body: &mut CastBody,
    caster: Entity,
    target: Option<Entity>,
    cost: f32,
    view: &TorrentView,
    tick_ms: u32,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> LiveTorrent {
    let hooked = start_torrent(body);
    let torrent = Torrent::new(cost, tick_ms, view, &mut draws.0, roll.0);
    roll.0 = torrent.roll;
    LiveTorrent {
        torrent,
        caster,
        target,
        hooked,
    }
}

/// Whether the caster called the torrent's hook since the last tick:
/// another cast's start replaced it, or the release flag is set, which also
/// ends the cast. The torrent then fizzles or ends.
pub fn hook_called(live: &mut LiveTorrent, body: &mut CastBody) -> bool {
    if !live.hooked {
        return false;
    }
    let called = if body.casting != TORRENT || body.cast_hook != Some(TORRENT_HOOK) {
        true
    } else if body.release {
        body.release = false;
        body.cast_hook = None;
        body.casting = 0;
        true
    } else {
        false
    };
    if called {
        live.hooked = false;
        live.torrent.release();
    }
    called
}

/// One tick of a torrent on its caster's body: the hook, then the tick
/// paying its drain.
pub fn tick_torrent(
    live: &mut LiveTorrent,
    body: &mut CastBody,
    view: &TorrentView,
    draws: &mut EffectDraws,
    roll: &mut EffectRoll,
) -> TorrentEvents {
    hook_called(live, body);
    let hooked = live.hooked;
    let mut short = false;
    live.torrent.roll = roll.0;
    let events = live.torrent.tick_paying(view, &mut draws.0, |cost| {
        let shortfall = spend_tick(body, cost);
        short |= shortfall;
        shortfall && hooked
    });
    roll.0 = live.torrent.roll;
    if short {
        live.hooked = false;
        body.cast_hook = None;
        body.casting = 0;
    }
    events
}

/// One tick of a running torrent's live points against `body`'s limb
/// spheres. Returns the pushes made.
pub fn strike_streams(torrent: &Torrent, body: &mut Ragdoll) -> usize {
    let character = body.drive.is_some();
    let mass = body_mass(body);
    let mut pushes = 0;
    for stream in &torrent.records[..torrent.count] {
        let mut k = stream.head;
        loop {
            let point = stream.points[k];
            if point.life < 1 {
                break;
            }
            let share = point.life as f32 / torrent.point_life as f32;
            for part_index in 0..body.parts.len() {
                let part = &body.parts[part_index];
                let inverse = part.inverse_mass;
                let (scale, push, _) = limb_terms(character, inverse, mass);
                let reach = torrent_reach(point.width, scale, torrent.margin);
                let world = part.transform();
                let here: Vec3 = part.positions.iter().copied().sum::<Vec3>() * 0.25;
                let before: Vec3 = part.previous.iter().copied().sum::<Vec3>() * 0.25;
                let velocity = (here - before) * CM;
                let spheres: Vec<(Vec3, f32)> = part
                    .spheres
                    .iter()
                    .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
                    .collect();
                for (s, (centre, radius)) in spheres.into_iter().enumerate() {
                    let d = point.at - centre;
                    let distance = d.length();
                    if distance >= reach + radius || distance <= 0.0 || distance.is_nan() {
                        continue;
                    }
                    let contact = centre + d * (radius / distance);
                    let impulse = limb_impulse(&point, contact, velocity, inverse, push, share);
                    if !impulse.is_finite() || impulse == Vec3::ZERO {
                        continue;
                    }
                    body.push_sphere(part_index, s, impulse / CM);
                    pushes += 1;
                }
            }
            k = if k == 0 { POINTS - 1 } else { k - 1 };
            if k == stream.head {
                break;
            }
        }
    }
    pushes
}

/// Ticks every torrent in the scene.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick_torrents(
    mut commands: Commands,
    mut torrents: Query<(Entity, &mut LiveTorrent)>,
    mut casters: Query<(
        &mut PowerCaster,
        Option<&Locomotor>,
        Option<&GlobalTransform>,
    )>,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    places: Query<&GlobalTransform>,
    cursor: Option<Res<CursorGround>>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
) {
    for (entity, mut live) in &mut torrents {
        let live = &mut *live;
        let target = live
            .target
            .and_then(|t| {
                bodies
                    .get(t)
                    .ok()
                    .and_then(|(_, body)| {
                        body.parts
                            .iter()
                            .find(|p| p.bone == CHEST)
                            .map(|p| Vec3::from(p.transform().translation) * CM)
                    })
                    .or_else(|| places.get(t).ok().map(|p| p.translation() * CM))
            })
            .or_else(|| {
                live.target
                    .is_none()
                    .then(|| cursor.as_deref().and_then(|c| c.0).map(|p| p * CM))
                    .flatten()
            });
        let mut scratch = CastBody::default();
        let mut caster = casters.get_mut(live.caster).ok();
        let facing = match caster.as_ref() {
            Some((_, Some(l), _)) => Vec3::new(l.yaw.sin(), 0.0, l.yaw.cos()),
            Some((_, None, Some(at))) => at.rotation() * Vec3::Z,
            _ => Vec3::Z,
        };
        let view = bodies
            .get(live.caster)
            .ok()
            .and_then(|(_, body)| torrent_view(body, facing, target));
        let body = match caster.as_mut() {
            Some((caster, _, _)) => &mut caster.body,
            None => {
                live.hooked = false;
                live.torrent.release();
                &mut scratch
            }
        };
        let Some(view) = view else {
            if live.hooked {
                body.cast_hook = None;
                body.casting = 0;
            }
            commands.entity(entity).despawn();
            continue;
        };
        tick_torrent(live, body, &view, &mut draws, &mut roll);
        if let Some((caster, _, _)) = caster.as_mut() {
            caster.sync_body();
        }
        if live.torrent.flags & RUNNING_FLAGS != 0 {
            for (other, mut body) in &mut bodies {
                if other == live.caster {
                    continue;
                }
                let pushes = strike_streams(&live.torrent, &mut body);
                if pushes == 0 {
                    continue;
                }
                body.touch(WAKE_TICKS);
                let mass = body_mass(&body);
                if let Some(drive) = body.drive.as_mut() {
                    for _ in 0..pushes {
                        drive.energy = energy_after(drive.energy, mass);
                    }
                }
            }
        }
        if live.torrent.builds_nothing() {
            if let Ok((mut caster, _, _)) = casters.get_mut(live.caster)
                && live.hooked
            {
                caster.body.cast_hook = None;
                caster.body.casting = 0;
                caster.sync_body();
            }
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;
    use crate::effects::torrent::{CHARGING, ENDING, FIZZLING, RUNNING};
    use crate::thaum::engine::Roll;

    fn view() -> TorrentView {
        TorrentView {
            target: Vec3::new(0.0, 150.0, 1000.0),
            hand_node: Vec3::new(0.0, 150.0, 0.0),
            hand: Vec3::new(0.0, 150.0, 0.0),
            facing: Vec3::Z,
            turned_aim: Vec3::Z,
        }
    }

    fn live(body: &mut CastBody) -> (LiveTorrent, EffectDraws, EffectRoll) {
        let mut draws = EffectDraws(Mt19937::new(1));
        let mut roll = EffectRoll(Roll(7));
        let torrent = new_torrent(
            body,
            Entity::PLACEHOLDER,
            None,
            0.1,
            &view(),
            12,
            &mut draws,
            &mut roll,
        );
        (torrent, draws, roll)
    }

    #[test]
    fn the_start_plays_the_motion_and_takes_the_hook() {
        let mut body = CastBody::default();
        let (torrent, _, roll) = live(&mut body);
        assert!(torrent.hooked);
        assert_eq!(body.casting, TORRENT);
        assert_eq!(body.cast_motion, Some(TORRENT_MOTION));
        assert_eq!(body.cast_hook, Some(TORRENT_HOOK));
        assert_eq!(torrent.torrent.state, CHARGING);
        assert_eq!(torrent.torrent.drain, 0.1 * 0.012);
        // The start's sixteen delays drew from the shared roll.
        assert_eq!(roll.0, torrent.torrent.roll);
        assert_ne!(roll.0, Roll(7));
    }

    #[test]
    fn a_later_cast_fizzles_a_charging_torrent_and_a_shortfall_ends_the_cast() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        assert!(casting::start_cast(&mut body, 0x22, 0x201));
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert!(!torrent.hooked);
        assert!(torrent.torrent.state < CHARGING);
        assert_eq!(body.casting, 0x201);

        let mut body = CastBody {
            focus: 0.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert_eq!(torrent.torrent.state, FIZZLING);
        assert_eq!((body.casting, body.cast_hook), (0, None));
    }

    #[test]
    fn a_running_torrent_keeps_the_hook_until_the_release_ends_it() {
        let mut body = CastBody {
            focus: 1.0,
            ..CastBody::default()
        };
        let (mut torrent, mut draws, mut roll) = live(&mut body);
        let mut ticks = 0;
        while torrent.torrent.state == CHARGING {
            tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
            ticks += 1;
            assert!(ticks < 200);
        }
        assert_eq!(torrent.torrent.state, RUNNING);
        assert!(torrent.hooked);
        assert_eq!(body.cast_hook, Some(TORRENT_HOOK));
        assert!(!torrent.torrent.builds_nothing());
        body.release = true;
        tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
        assert_eq!(torrent.torrent.state, ENDING);
        assert_eq!((body.casting, body.cast_hook, body.release), (0, None, false));
        let mut ticks = 0;
        while !torrent.torrent.builds_nothing() {
            tick_torrent(&mut torrent, &mut body, &view(), &mut draws, &mut roll);
            ticks += 1;
            assert!(ticks < 200);
        }
    }
}
