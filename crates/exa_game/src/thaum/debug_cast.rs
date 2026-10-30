//! A debug helper (`--debug-cast slot@frame[~release]`): at a frame after
//! the load the player casts the power in a bar slot, and at the release
//! frame uses it again (which lets a held power such as Vanish go). The
//! cursor's ground point is put 1.5 m beyond the player, away from the
//! camera, on the cast frame, so a power that needs it (the Long Door) has
//! a destination. With `--fixed-frames` the runs repeat tick for tick, so a
//! screenshot can catch an effect at a chosen moment.

use bevy::prelude::*;

use super::cast::{BarAction, BarCommand, PowerCaster};
use super::tree::word;
use crate::player::{CursorGround, PlayerBody};

/// One scripted cast: the bar slot (0-based, the order of `--powers`), the
/// frame after the load it is cast on and the frame it is used again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugCast {
    pub slot: usize,
    pub at: u32,
    pub release: Option<u32>,
}

impl DebugCast {
    /// Reads `slot@frame` or `slot@frame~release`.
    pub fn parse(text: &str) -> Option<Self> {
        let (slot, rest) = text.split_once('@')?;
        let (at, release) = match rest.split_once('~') {
            Some((at, release)) => (at, Some(release.trim().parse().ok()?)),
            None => (rest, None),
        };
        Some(Self {
            slot: slot.trim().parse().ok()?,
            at: at.trim().parse().ok()?,
            release,
        })
    }
}

/// The scripted casts.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct DebugCasts(pub Vec<DebugCast>);

/// The bar commands a frame sends: the cast's open, pick (unless the slot
/// is already the chosen one, which a pick would unchoose), close and use;
/// a release's use alone (the slot is still picked).
pub fn commands_at(casts: &[DebugCast], frame: u32, chosen: Option<usize>) -> Vec<BarAction> {
    let mut out = Vec::new();
    for cast in casts {
        if cast.at == frame {
            out.push(BarAction::Toggle);
            if chosen != Some(cast.slot) {
                out.push(BarAction::Pick(cast.slot));
            }
            out.extend([BarAction::Toggle, BarAction::Use]);
        }
        if cast.release == Some(frame) {
            out.push(BarAction::Use);
        }
    }
    out
}

/// Sends the scripted casts' bar commands on their frames.
pub(crate) fn run_debug_casts(
    casts: Res<DebugCasts>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    players: Query<(Entity, &GlobalTransform, &PowerCaster), With<PlayerBody>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    ground: Option<ResMut<CursorGround>>,
    mut bars: MessageWriter<BarCommand>,
    mut frames: Local<u32>,
) {
    if casts.0.is_empty() || loading.is_some() {
        return;
    }
    *frames += 1;
    let frame = *frames;
    let Some((player, at, caster)) = players.iter().next() else {
        return;
    };
    let chosen = caster
        .record
        .slots
        .iter()
        .position(|slot| slot & word::CHOSEN != 0);
    let actions = commands_at(&casts.0, frame, chosen);
    if actions.is_empty() {
        return;
    }
    let away = cameras
        .iter()
        .next()
        .map(|camera| {
            let mut d = at.translation() - camera.translation();
            d.y = 0.0;
            d.normalize_or(Vec3::X)
        })
        .unwrap_or(Vec3::X);
    let point = at.translation() + away * 1.5;
    if let Some(mut ground) = ground {
        ground.0 = Some(point);
    }
    info!("debug cast: frame {frame}: {actions:?}, cursor ground {point:?}");
    for action in actions {
        bars.write(BarCommand {
            caster: player,
            action,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_debug_cast_reads_its_slot_frame_and_release() {
        assert_eq!(
            DebugCast::parse("1@120~300"),
            Some(DebugCast {
                slot: 1,
                at: 120,
                release: Some(300)
            })
        );
        assert_eq!(
            DebugCast::parse("0@60"),
            Some(DebugCast {
                slot: 0,
                at: 60,
                release: None
            })
        );
        assert_eq!(DebugCast::parse("0"), None);
        assert_eq!(DebugCast::parse("a@1"), None);
        let casts = [DebugCast::parse("1@10~20").unwrap()];
        assert_eq!(commands_at(&casts, 9, None), vec![]);
        assert_eq!(
            commands_at(&casts, 10, Some(0)),
            vec![
                BarAction::Toggle,
                BarAction::Pick(1),
                BarAction::Toggle,
                BarAction::Use
            ]
        );
        // A slot already chosen is not picked again (that would unchoose it).
        assert_eq!(
            commands_at(&casts, 10, Some(1)),
            vec![BarAction::Toggle, BarAction::Toggle, BarAction::Use]
        );
        assert_eq!(commands_at(&casts, 20, None), vec![BarAction::Use]);
    }
}
