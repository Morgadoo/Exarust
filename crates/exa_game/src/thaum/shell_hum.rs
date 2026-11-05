//! The hum a kinesis's force shell plays while it holds a body.
//!
//! - **The buffer**: the kinesis's pick loads the `thmfrc_knsdr` buffer
//!   (once a session) and names it on the shell it puts on the body
//!   ([`super::hold_live::ShellHum`]). No other shell names one: the Shell
//!   power's, the throw's grab and the strikes auras stay silent.
//! - **The start**: when the shell joins the scene it takes a voice for
//!   the buffer at the shell node's place: volume 0, pitch 1, looping, its
//!   start `r·0.8` of the buffer's length into it (`r` the effect stream's
//!   draw; `r·0.8` first, then the length, in double).
//! - **Each frame** the shell's render step moves the voice to the node's
//!   place and writes its volume and pitch ([`crate::thaum::shell_pair`]):
//!   0.1 and 0.75 at rest, rising along `x(2 − x)` (`x` four times the shown
//!   amount) over the first quarter of the opening, and
//!   `0.2·(2 − |1.5 − 3.5·s|)` and `0.75·s` while it closes.
//! - **The end**: the voice is freed with the shell.
//!
//! A kinesis that hands its body to a throw hands the shell, and its hum
//! plays on until the throw drops the shell.
//!
//! The node's place is its box's centre in the world: the held body's
//! meshes' box ([`super::effect_draw::RigidFit`]) carried by its first
//! part, else that part's origin before the box is fitted.
//!
//! Evidence: the kinesis's pick callback, the force shell's constructor,
//! scene-join callback and render step, read from `Exanima.exe`.

use std::collections::HashMap;

use bevy::prelude::*;
use exa_assets::Game;
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use exa_physics::body::Ragdoll;

use super::cast::LiveShellAura;
use super::effect_draw::{RigidFit, part_frame};
use super::force::KINESIS_HUM;
use super::hold_live::{RigidAuraOf, ShellHum};

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The hum's start, as a share of its buffer's length (times the draw).
pub const HUM_START_SHARE: f64 = 0.8;

/// The hum's start in seconds: the draw times 0.8, then the buffer's
/// length, in double.
pub fn hum_offset(length_secs: f32, roll: f64) -> f32 {
    (f64::from(length_secs) * (roll * HUM_START_SHARE)) as f32
}

/// A rigid shell node's place (cm): its fitted box's centre carried by the
/// body's first part, else that part's origin.
pub fn node_place(body: &Ragdoll, fit: Option<&RigidFit>) -> Option<Vec3> {
    let frame = part_frame(body.parts.first()?.transform());
    let centre = fit.map_or(Vec3::ZERO, |fit| fit.centre * CM);
    Some(
        frame.translation
            + frame.rows[0] * centre.x
            + frame.rows[1] * centre.y
            + frame.rows[2] * centre.z,
    )
}

/// What the hums take from the game (none in tests without it).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct HumAudio<'w> {
    game: Option<Res<'w, Game>>,
    bank: Option<ResMut<'w, SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, AudioEnvironment>>,
}

/// The hum buffer, loaded once (`None` inside when it failed to load), and
/// each humming shell's voice, by the shell's entity.
#[derive(Default)]
pub(crate) struct HumVoices {
    buffer: Option<Option<u32>>,
    voices: HashMap<Entity, VoiceHandle>,
}

/// Each frame, after the shells' render steps: a humming shell's voice
/// starts on its first frame and follows its node and its levels; a shell
/// that has left the scene frees its voice.
#[allow(clippy::type_complexity)]
pub(crate) fn sound_shell_hums(
    shells: Query<(
        Entity,
        &LiveShellAura,
        &ShellHum,
        Option<&RigidAuraOf>,
        Option<&RigidFit>,
    )>,
    bodies: Query<&Ragdoll>,
    mut audio: HumAudio,
    mut state: Local<HumVoices>,
) {
    let (Some(game), Some(bank), Some(sources), Some(env)) = (
        audio.game.as_deref(),
        audio.bank.as_deref_mut(),
        audio.sources.as_deref_mut(),
        audio.env.as_deref_mut(),
    ) else {
        return;
    };
    let gone: Vec<Entity> = state
        .voices
        .keys()
        .copied()
        .filter(|&e| !shells.contains(e))
        .collect();
    for entity in gone {
        if let Some(handle) = state.voices.remove(&entity) {
            env.free_handle(bank, handle);
            info!("shell hum {entity:?}: its shell is gone, the voice freed");
        }
    }
    if !env.is_live() {
        return;
    }
    for (entity, aura, hum, of, fit) in &shells {
        let Some(at) = of
            .and_then(|of| bodies.get(of.0).ok())
            .and_then(|body| node_place(body, fit))
        else {
            continue;
        };
        let handle = match state.voices.get(&entity) {
            Some(&handle) => handle,
            None => {
                let Some(buffer) = *state
                    .buffer
                    .get_or_insert_with(|| bank.load_buffer(game, sources, KINESIS_HUM))
                else {
                    continue;
                };
                let handle = env.new_handle();
                state.voices.insert(entity, handle);
                if env.play_buffer_attached(buffer, handle, at / CM, 0.0, 1.0, voice_flags::LOOPING)
                    && let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id))
                {
                    voice.offset = hum_offset(bank.buffer_secs(buffer), hum.roll);
                    info!(
                        "shell hum {entity:?}: {KINESIS_HUM} loop at {at:?} cm, start {:.3} s of {:.3} s (draw {:.4})",
                        voice.offset,
                        bank.buffer_secs(buffer),
                        hum.roll
                    );
                }
                handle
            }
        };
        if let Some(voice) = env.handle_voice(handle).and_then(|id| env.voice_mut(id)) {
            voice.position = at / CM;
            voice.gain = aura.0.hum.first;
            voice.pitch = aura.0.hum.second;
            debug!(
                "shell hum {entity:?}: state {}, shown {:.3}: volume {:.3}, pitch {:.3} at {at:?} cm",
                aura.0.state, aura.0.shown, voice.gain, voice.pitch
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hum_starts_a_share_of_its_length_in() {
        // The draw times 0.8 first, then the length, in double.
        assert_eq!(hum_offset(2.5, 0.5), 1.0);
        let r = 0.123_456_789_f64;
        assert_eq!(hum_offset(3.3, r), (f64::from(3.3f32) * (r * 0.8)) as f32);
        assert_eq!(KINESIS_HUM, "thmfrc_knsdr");
    }

    #[test]
    fn the_node_stands_at_its_box_centre() {
        use bevy::math::Affine3A;
        use exa_physics::body::PartSetup;
        let body = Ragdoll::new(&[PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_rotation_translation(
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                Vec3::new(1.0, 2.0, 3.0),
            ),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            spheres: None,
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        }]);
        assert_eq!(
            node_place(&body, None),
            Some(Vec3::new(100.0, 200.0, 300.0))
        );
        let fit = RigidFit {
            centre: Vec3::new(0.1, 0.0, 0.0),
            half: Vec3::ONE,
        };
        let at = node_place(&body, Some(&fit)).unwrap();
        // The part's x axis is the world's y after the quarter turn.
        assert!((at - Vec3::new(100.0, 210.0, 300.0)).length() < 1e-3);
    }
}
