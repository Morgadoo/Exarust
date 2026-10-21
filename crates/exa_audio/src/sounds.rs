//! The named sound sets of the body and its blows: the swing whoosh, the
//! second wind's heal, the hit-sound classes of armour and flesh, and the
//! clothing and golem foley loops.
//!
//! Evidence: the part's sound set names (`SwingHA`, `StmHeal01`, `HitGlm`,
//! `FoleyPlH`, `Golem_Foley`) and the pack's numbered variants: `swingha`
//! 6, `stmheal` 1, `bdycol` 6, `skelhit` 6, `hitclt` 1, `hitltr` 4,
//! `hitchn` 3, `hitplt` 8, `hitglm` 6, `foleyplh` 9, `golem_foley` 6. The
//! class-to-set mapping follows the apparel doc (0 flesh, 1 cloth, 2
//! leather, 3 mail, 4 plate, 6 golem); the flesh and skeleton defaults are
//! `BdyCol` and `SkelHit` (a guess: their runtime pointers are built
//! elsewhere).

use bevy::prelude::*;

use exa_assets::Game;
use exa_combat::sounds::{FoleyKind, HitSound};

use crate::{AudioEnvironment, SoundBank, SoundSet};

/// A swing's whoosh (`SwingHA`), six variants.
pub const SWING: SoundSet<'static> = SoundSet::new("swingha", 6);
/// The second wind's heal (`StmHeal01`), one sound.
pub const HEAL: SoundSet<'static> = SoundSet::new("stmheal", 1);
/// The flesh hit default (`BdyCol`).
pub const HIT_FLESH: SoundSet<'static> = SoundSet::new("bdycol", 6);
/// The skeleton hit default (`SkelHit`).
pub const HIT_SKELETON: SoundSet<'static> = SoundSet::new("skelhit", 6);
/// The golem's hit sound (`HitGlm`).
pub const HIT_GOLEM: SoundSet<'static> = SoundSet::new("hitglm", 6);
/// Cloth (`hitclt`), leather (`hitltr`), mail (`hitchn`) and plate
/// (`hitplt`).
pub const HIT_CLOTH: SoundSet<'static> = SoundSet::new("hitclt", 1);
pub const HIT_LEATHER: SoundSet<'static> = SoundSet::new("hitltr", 4);
pub const HIT_MAIL: SoundSet<'static> = SoundSet::new("hitchn", 3);
pub const HIT_PLATE: SoundSet<'static> = SoundSet::new("hitplt", 8);
/// The clothing foley (`FoleyPlH`) and the golem's (`Golem_Foley`).
pub const FOLEY_PLAYER: SoundSet<'static> = SoundSet::new("foleyplh", 9);
pub const FOLEY_GOLEM: SoundSet<'static> = SoundSet::new("golem_foley", 6);

/// The set a part's material plays, or none when the class has no set.
pub fn hit_set(sound: HitSound) -> Option<SoundSet<'static>> {
    match sound {
        HitSound::Flesh => Some(HIT_FLESH),
        HitSound::Skeleton => Some(HIT_SKELETON),
        HitSound::Golem => Some(HIT_GOLEM),
        // The layer's stored classes: 2 cloth, 3 leather, 4 mail, 5 plate;
        // 0/1 are flesh and bone (the defaults).
        HitSound::Class(0 | 1) => Some(HIT_FLESH),
        HitSound::Class(2) => Some(HIT_CLOTH),
        HitSound::Class(3) => Some(HIT_LEATHER),
        HitSound::Class(4) => Some(HIT_MAIL),
        HitSound::Class(5) => Some(HIT_PLATE),
        HitSound::Class(6) => Some(HIT_GOLEM),
        HitSound::Class(_) => None,
    }
}

/// The set a foley loop plays, or none.
pub fn foley_set(kind: FoleyKind) -> Option<SoundSet<'static>> {
    match kind {
        FoleyKind::None => None,
        FoleyKind::Player | FoleyKind::Class(_) => Some(FOLEY_PLAYER),
        FoleyKind::Golem => Some(FOLEY_GOLEM),
    }
}

/// A request to play the second wind's heal sound at a body.
#[derive(Message, Debug, Clone, Copy)]
pub struct HealSound {
    pub position: Vec3,
}

/// Plays a heal request at the original's volume 0.1 and pitch 1.
pub(crate) fn heal_sounds(
    game: Option<Res<Game>>,
    mut requests: MessageReader<HealSound>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    mut set: Local<Option<u32>>,
) {
    let Some(game) = game else { return };
    for request in requests.read() {
        let id = *set.get_or_insert_with(|| bank.load_set(&game, &mut sources, HEAL.prefix));
        env.play_set(&mut bank, id, request.position, 0.1, 1.0, 0);
    }
}

/// Plays a swing's whoosh when a fighter's strike window opens, at its
/// primary weapon's swing volume and pitch (W25, W26).
pub(crate) fn swing_sounds(
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut env: ResMut<AudioEnvironment>,
    swings: Query<(Entity, &exa_combat::swing::SwingState, &GlobalTransform)>,
    blades: Query<&exa_combat::Blade>,
    mut open: Local<bevy::platform::collections::HashMap<Entity, bool>>,
) {
    let Some(game) = game else { return };
    for (entity, state, gt) in &swings {
        let now_open = state.open[0] || state.open[1];
        let was = open.insert(entity, now_open).unwrap_or(false);
        if !now_open || was {
            continue;
        }
        let Some(blade) = blades.iter().find(|b| b.wielder == entity) else {
            continue;
        };
        let set = bank.load_set(&game, &mut sources, SWING.prefix);
        let position = gt.translation() + Vec3::Y;
        env.play_set(
            &mut bank,
            set,
            position,
            blade.record.swing_volume,
            blade.record.swing_pitch,
            0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every class maps to a shipped set, and unknown classes do not.
    #[test]
    fn the_hit_and_foley_sets_cover_the_classes() {
        assert_eq!(hit_set(HitSound::Flesh), Some(HIT_FLESH));
        assert_eq!(hit_set(HitSound::Skeleton), Some(HIT_SKELETON));
        assert_eq!(hit_set(HitSound::Golem), Some(HIT_GOLEM));
        assert_eq!(hit_set(HitSound::Class(2)), Some(HIT_CLOTH));
        assert_eq!(hit_set(HitSound::Class(3)), Some(HIT_LEATHER));
        assert_eq!(hit_set(HitSound::Class(4)), Some(HIT_MAIL));
        assert_eq!(hit_set(HitSound::Class(5)), Some(HIT_PLATE));
        assert_eq!(hit_set(HitSound::Class(6)), Some(HIT_GOLEM));
        assert_eq!(hit_set(HitSound::Class(9)), None);
        assert_eq!(foley_set(FoleyKind::None), None);
        assert_eq!(foley_set(FoleyKind::Player), Some(FOLEY_PLAYER));
        assert_eq!(foley_set(FoleyKind::Class(2)), Some(FOLEY_PLAYER));
        assert_eq!(foley_set(FoleyKind::Golem), Some(FOLEY_GOLEM));
    }
}
