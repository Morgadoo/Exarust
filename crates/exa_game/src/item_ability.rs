//! Item abilities: a held item whose weapon section names an ability, and
//! the burning blade it gives.
//!
//! A weapon section's body carries an ability type (the word at byte
//! [`ABILITY_AT`]) and, from byte [`SOCKET_AT`], its crystal sockets: the
//! first socket's kind in the low byte and the crystal's charge in the high
//! one ([`exa_items::item_gui::Sockets`]). In the shipped object database
//! four items have a type: 1 "UB5 FlangedMace", 2 "Swrd Self Sharp", 3
//! "Sword GSF 01" and 4 "Sceptre A"; the story's seventh level places the
//! only copy of "Sword GSF 01" in its local database. No routine writes an
//! ability's state: it comes from the socket a crystal is dropped into.
//!
//! **Type 3, the burning blade.** Every game second, while either hand holds
//! an item with an ability, the original runs each held item's ability
//! check. For type 3 the check ([`fire_row::held_tick`]) lights the blade
//! when its first socket holds a Power Crystal C (kind 4) whose charge byte
//! is below 254 and no live flame row is on it, and puts a burning blade out
//! once the crystal is gone. Nothing drains a kind-4 crystal, so a socketed
//! blade burns until the crystal is taken out.
//!
//! - **Lighting** raises the item's lit flag ([`LitBlade`]), puts a flame
//!   row on the item along its own +Y from 30 to 115 cm, fading in
//!   ([`fire_row::ignition`]), asks the moveset choice for the "Flaming"
//!   overlay ([`HeldModifier`]) and re-runs the player light's switch, which
//!   a lit blade in the first hand turns off.
//! - **Putting out** sets the row's extinguish flag (it fades out and leaves
//!   the scene), clears the lit flag and the overlay, and re-runs the
//!   switch.
//! - **A hit** with a burning blade plays `FireSizzleA` at the contact,
//!   volume 0.3 and pitch `0.8 + 0.2·u` ([`fire_row::sizzle`]).
//!
//! The row's light (warm white, 800 cm, softness 4·j, five at full fade)
//! sits at the middle of the line, its loop `FireS02` follows the row's
//! speed and fade, and its ribbon is drawn with `_FlameMat01`
//! ([`exa_render::flame_row`]) over the `flameletsa3` picture the material's
//! built-in record names.
//!
//! Exarust runs the check for the player's two hand slots, from the live
//! object database ([`LiveObjects`]); the original runs it for every
//! character. Types 1 and 2 (a lightning mace and a self-sharpening sword)
//! are not ported.
//!
//! Debug tool: `--flame-sword` (the binary's flag) gives the player "Sword
//! GSF 01" and drops a Power Crystal C into its first socket through the
//! item GUI's socket rule.
//!
//! Evidence: the held-item ability install, its once-a-second tick, the
//! type-3 check, lighting and putting out, the hit callback, the flame
//! row's node, the item GUI's socket drop and the player light's switch,
//! read from `Exanima.exe` and checked against the disassembly.

use std::collections::HashMap;

use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use exa_assets::live_objects::LiveObjects;
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader};
use exa_audio::{AudioEnvironment, SoundBank, VoiceHandle, voice_flags};
use exa_character::held::HeldModifier;
use exa_combat::attack::Slot;
use exa_combat::{Blade, Hit};
use exa_formats::rsq::MovesetConfig;
use exa_items::possession::Possessions;
use exa_render::flame_row::{FlameRowMaterial, gradient_image};

use crate::effects::fire_row::{self, AttachedRow, Flame, HeldFire, ItemAbility};
use crate::player::PlayerBody;

/// The weapon section's tag.
pub const WEAPON_SECTION: u32 = 0x4000;
/// The ability type's word in the weapon section's body.
pub const ABILITY_AT: usize = 0x44;
/// The first socket's word (kind, then charge) in the weapon section's body.
pub const SOCKET_AT: usize = 0x48;
/// The ability type of a blade a crystal sets alight.
pub const BURNING_BLADE: u32 = fire_row::BURNING_ABILITY;
/// How often the held items' abilities are checked (ms of game time).
pub const TICK_MS: u64 = 1000;
/// "Sword GSF 01", the one burning blade of the shipped database.
pub const FLAME_SWORD: u32 = 0x487;
/// Its model in `Objects.rpk`.
pub const FLAME_SWORD_MODEL: &str = "Sword GSF 01";
/// The flame row's material picture (`_FlameMat01`'s built-in record).
pub const FLAME_TEXTURE: &str = "flameletsa3";
/// Where along the item's +Y the row's light sits (cm): the line's middle.
pub const LIGHT_AT_CM: f32 = (fire_row::START_CM + fire_row::END_CM) * 0.5;

pub struct ItemAbilityPlugin;

impl Plugin for ItemAbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AbilityClock>()
            .init_resource::<FlameRowVoices>()
            .add_systems(
                Update,
                (
                    debug_flame_sword,
                    ability_tick,
                    sizzle_on_hit,
                    (flame_row_lights, flame_row_draws, flame_row_loops)
                        .after(fire_row::frame_rows),
                ),
            );
    }
}

/// An item's ability: its type word and its first socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ability {
    pub kind: u32,
    pub socket: ItemAbility,
}

/// The ability a weapon section's body names, if it is long enough to hold
/// one.
pub fn ability_of(body: &[u8]) -> Option<Ability> {
    let kind = u32::from_le_bytes(body.get(ABILITY_AT..ABILITY_AT + 4)?.try_into().ok()?);
    let word = u16::from_le_bytes(body.get(SOCKET_AT..SOCKET_AT + 2)?.try_into().ok()?);
    Some(Ability {
        kind,
        socket: ItemAbility {
            kind: u32::from(word),
            fuel: (word >> 8) as u8,
        },
    })
}

/// The item's render lit flag: its flame row burns. Kept on the held item
/// (the [`Blade`]) with the row it carries.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LitBlade {
    pub row: Entity,
}

/// The light a flame row shows, at the middle of its line.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlameRowLight {
    pub row: Entity,
}

/// A flame row's ribbon draw.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlameRowDraw {
    pub row: Entity,
}

/// The ability tick's clock: the game time (ms) a player's next check is
/// due, as the original keeps one per character.
#[derive(Resource, Debug, Default)]
pub struct AbilityClock {
    pub next_ms: HashMap<Entity, u64>,
}

/// Debug tool (`--flame-sword`): the player holds "Sword GSF 01" with a
/// Power Crystal C in its first socket. Applied once the player and the
/// live database are there.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugFlameSword {
    pub applied: bool,
}

/// What the check does to one held item this tick.
pub fn check(ability: Option<Ability>, row: AttachedRow) -> HeldFire {
    match ability {
        Some(a) if a.kind == BURNING_BLADE => fire_row::held_tick(Some(a.socket), row).action,
        _ => HeldFire::None,
    }
}

/// Whether the player light's lit-blade rule puts it out: an item of type 3
/// in either hand while the first held item burns.
pub fn blade_puts_light_out(kinds: [Option<u32>; 2], first_lit: bool) -> bool {
    kinds
        .iter()
        .any(|&k| crate::player_light::weapon_puts_out(k.map(|k| k as u8), first_lit))
}

fn debug_flame_sword(
    debug: Option<ResMut<DebugFlameSword>>,
    game: Option<Res<Game>>,
    mut players: Query<&mut Possessions, With<PlayerBody>>,
    objects: Option<ResMut<LiveObjects>>,
) {
    let (Some(mut debug), Some(game), Some(mut objects)) = (debug, game, objects) else {
        return;
    };
    if debug.applied {
        return;
    }
    let Some(mut possessions) = players.iter_mut().next() else {
        return;
    };
    if !objects.ensure_base(&game.0) {
        return;
    }
    debug.applied = true;
    possessions.out[0].id = FLAME_SWORD;
    let Some(body) = objects.db().section(FLAME_SWORD, WEAPON_SECTION) else {
        warn!("--flame-sword: the database has no \"{FLAME_SWORD_MODEL}\" weapon section");
        return;
    };
    let mut sockets = exa_items::item_gui::Sockets::of_section(body, SOCKET_AT);
    let dropped = sockets.drop_crystal(0, exa_items::possession::crystal::C, 0, false);
    let [low, high] = sockets.0[0].to_le_bytes();
    objects.write_section_byte(FLAME_SWORD, WEAPON_SECTION, SOCKET_AT, low);
    objects.write_section_byte(FLAME_SWORD, WEAPON_SECTION, SOCKET_AT + 1, high);
    let ability = objects
        .db()
        .section(FLAME_SWORD, WEAPON_SECTION)
        .and_then(ability_of);
    info!(
        "--flame-sword: the player holds {FLAME_SWORD:#x} \"{FLAME_SWORD_MODEL}\"; Power Crystal C into its first socket ({dropped:?}): {ability:?}"
    );
}

/// The row a held item carries and whether it is on its way out.
fn attached_row(lit: Option<&LitBlade>, rows: &Query<&mut Flame>) -> AttachedRow {
    match lit.and_then(|l| rows.get(l.row).ok()) {
        Some(flame) => AttachedRow {
            exists: true,
            extinguishing: flame.row().flags & fire_row::EXTINGUISHING != 0,
        },
        None => AttachedRow::NONE,
    }
}

/// Once a game second, each of the player's held items' ability check.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn ability_tick(
    mut commands: Commands,
    mut clock: ResMut<AbilityClock>,
    time: Option<Res<Time<Virtual>>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut players: Query<(Entity, &Possessions, Option<&mut HeldModifier>), With<PlayerBody>>,
    objects: Option<Res<LiveObjects>>,
    blades: Query<(Entity, &Blade, Option<&LitBlade>)>,
    mut rows: Query<&mut Flame>,
    mut seed: Local<u32>,
) {
    let (Some(time), Some(objects)) = (time, objects) else {
        return;
    };
    if loading.is_some() {
        return;
    }
    let now = time.elapsed().as_millis() as u64;
    for (player, possessions, modifier) in &mut players {
        let next = clock.next_ms.entry(player).or_insert(0);
        if now < *next {
            continue;
        }
        *next = now + TICK_MS;
        let abilities = [0, 1].map(|slot| {
            let id = possessions.out[slot].id;
            (id != 0)
                .then(|| objects.db().section(id, WEAPON_SECTION))
                .flatten()
                .and_then(ability_of)
        });
        // Nothing held has an ability: no check runs (the original's list
        // is empty).
        if abilities.iter().all(|a| a.is_none_or(|a| a.kind == 0)) {
            continue;
        }
        let mut flaming = modifier.as_ref().is_some_and(|m| m.0 != 0);
        for (slot, ability) in abilities.into_iter().enumerate() {
            let want = if slot == 0 { Slot::First } else { Slot::Second };
            let Some((blade, _, lit)) = blades
                .iter()
                .find(|(_, b, _)| b.wielder == player && b.slot == want)
            else {
                continue;
            };
            let row = attached_row(lit, &rows);
            match check(ability, row) {
                HeldFire::None => {}
                HeldFire::Ignite => {
                    *seed = seed.wrapping_add(1);
                    let lit = fire_row::ignition();
                    let row = commands
                        .spawn((
                            Name::new("FlameRow"),
                            Flame::new(lit.start_cm, lit.end_cm, lit.flags, *seed),
                            Transform::IDENTITY,
                            ChildOf(blade),
                        ))
                        .id();
                    commands.spawn((
                        Name::new("FlameRowLight"),
                        FlameRowLight { row },
                        exa_world::light_bundle(&fire_row::light_record()),
                        Transform::from_xyz(0.0, LIGHT_AT_CM / 100.0, 0.0),
                        ChildOf(blade),
                    ));
                    commands.entity(blade).insert(LitBlade { row });
                    flaming = true;
                    info!(
                        "item ability: slot {slot} {:#x} lights its blade (socket {:#06x}, flame row {row:?})",
                        possessions.out[slot].id,
                        ability.map_or(0, |a| a.socket.kind)
                    );
                }
                HeldFire::Extinguish => {
                    if let Some(lit) = lit
                        && let Ok(mut flame) = rows.get_mut(lit.row)
                    {
                        flame.extinguish();
                    }
                    commands.entity(blade).remove::<LitBlade>();
                    flaming = false;
                    info!(
                        "item ability: slot {slot} {:#x} puts its blade out",
                        possessions.out[slot].id
                    );
                }
            }
        }
        let wanted = if flaming { MovesetConfig::FLAMING.0 } else { 0 };
        match modifier {
            Some(mut m) => {
                m.set_if_neq(HeldModifier(wanted));
            }
            None if wanted != 0 => {
                commands.entity(player).insert(HeldModifier(wanted));
            }
            None => {}
        }
    }
}

/// A hit with a burning blade sizzles at the contact.
fn sizzle_on_hit(
    mut hits: MessageReader<Hit>,
    lit: Query<(), With<LitBlade>>,
    mut bank: ResMut<SoundBank>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
) {
    for hit in hits.read() {
        if !lit.contains(hit.striker) {
            continue;
        }
        let sizzle = fire_row::sizzle(f64::from(bank.random_unit()));
        info!(
            "item ability: a burning blade's hit sizzles at {:?} (volume {}, pitch {:.3})",
            hit.point, sizzle.volume, sizzle.pitch
        );
        sounds.write(exa_audio::PointSounded {
            name: fire_row::SIZZLE_SOUND.into(),
            position: hit.point,
            volume: sizzle.volume,
            pitch: sizzle.pitch,
            flags: 0,
        });
    }
}

/// Each row's light follows its row; it leaves with it.
fn flame_row_lights(
    mut commands: Commands,
    rows: Query<&Flame>,
    mut lights: Query<(Entity, &FlameRowLight, &mut PointLight)>,
) {
    for (entity, light, mut point) in &mut lights {
        let Ok(flame) = rows.get(light.row) else {
            commands.entity(entity).despawn();
            continue;
        };
        let shown = flame.row().light();
        point.intensity = shown.intensity.max(0.0) * exa_world::LUMENS_PER_INTENSITY;
        point.radius = exa_world::shadow_radius(shown.softness);
    }
}

/// The flame material's picture and palette, loaded once.
#[derive(Resource)]
struct FlameRowAssets {
    material: Handle<FlameRowMaterial>,
}

/// Each row's ribbon, rebuilt every frame from the camera and the tick's
/// fraction, drawn additively with `_FlameMat01`.
#[allow(clippy::too_many_arguments)]
fn flame_row_draws(
    mut commands: Commands,
    assets: Option<Res<FlameRowAssets>>,
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut exa_materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FlameRowMaterial>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
    fixed: Res<Time<Fixed>>,
    time: Res<Time>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    rows: Query<(Entity, &Flame)>,
    mut draws: Query<(Entity, &FlameRowDraw, &Mesh3d)>,
) {
    if rows.is_empty() && draws.is_empty() {
        return;
    }
    let assets = match assets {
        Some(assets) => assets.material.clone(),
        None => {
            let Some(game) = game else { return };
            let mut loader = ModelLoader {
                game: &game.0,
                cache: &mut cache,
                meshes: &mut meshes,
                materials: &mut exa_materials,
                images: &mut images,
                bc_supported: exa_assets::model::bc_supported(support.as_deref()),
            };
            let Some((picture, _)) = loader.texture(FLAME_TEXTURE, ColorSpace::Srgb) else {
                warn!("flame row: no {FLAME_TEXTURE} picture");
                return;
            };
            let gradient = images.add(gradient_image());
            let material = materials.add(FlameRowMaterial::new(picture, gradient));
            commands.insert_resource(FlameRowAssets {
                material: material.clone(),
            });
            material
        }
    };
    let Some(camera) = cameras
        .iter()
        .filter(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation() * 100.0)
        .next()
    else {
        return;
    };
    let t = fixed.overstep_fraction();
    let now_ms = time.elapsed().as_millis() as u32;
    let mut drawn = HashMap::new();
    for (entity, draw, mesh) in &mut draws {
        match rows.get(draw.row) {
            Ok((_, flame)) => {
                drawn.insert(draw.row, ());
                if let Some(mut m) = meshes.get_mut(&mesh.0) {
                    *m = flame.row().ribbon_mesh(camera, t);
                }
            }
            Err(_) => commands.entity(entity).despawn(),
        }
    }
    // One material per scene: the animation follows the first row's fade.
    if let Some((_, flame)) = rows.iter().next()
        && let Some(mut m) = materials.get_mut(&assets)
    {
        m.set_animation(fire_row::texture_animation(now_ms, flame.row().fade));
    }
    for (row, flame) in &rows {
        if drawn.contains_key(&row) || flame.row().removed() {
            continue;
        }
        commands.spawn((
            Name::new("FlameRowDraw"),
            FlameRowDraw { row },
            Mesh3d(meshes.add(flame.row().ribbon_mesh(camera, t))),
            MeshMaterial3d(assets.clone()),
            Transform::IDENTITY,
            NoFrustumCulling,
            // The ribbon runs through its own light: it neither casts nor
            // takes shadows.
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

/// The looping voice each row holds.
#[derive(Resource, Default)]
struct FlameRowVoices(HashMap<Entity, (VoiceHandle, Option<u32>)>);

/// Each row's `FireS02` loop: started once, then moved to the line's
/// middle and levelled from the row's sound every frame; freed when the
/// row leaves.
fn flame_row_loops(
    game: Option<Res<Game>>,
    mut bank: ResMut<SoundBank>,
    mut env: ResMut<AudioEnvironment>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut voices: ResMut<FlameRowVoices>,
    rows: Query<(Entity, &Flame)>,
) {
    let gone: Vec<Entity> = voices
        .0
        .keys()
        .copied()
        .filter(|&e| !rows.contains(e))
        .collect();
    for entity in gone {
        if let Some((handle, buffer)) = voices.0.remove(&entity) {
            env.free_handle(&mut bank, handle);
            if let Some(buffer) = buffer {
                bank.release_buffer(buffer);
            }
        }
    }
    let Some(game) = game else { return };
    if !env.is_live() {
        return;
    }
    for (entity, flame) in &rows {
        let row = flame.row();
        if row.removed() || !row.has_sound {
            continue;
        }
        let (handle, buffer) = *voices.0.entry(entity).or_insert_with(|| {
            (
                env.new_handle(),
                bank.load_buffer(&game, &mut sources, fire_row::LOOP_SOUND),
            )
        });
        let Some(buffer) = buffer else { continue };
        let sound = row.sound();
        let position = (row.start + row.end) * 0.5 / 100.0;
        match env.handle_voice(handle) {
            Some(id) => {
                if let Some(v) = env.voice_mut(id) {
                    v.position = position;
                    v.gain = sound.volume.max(0.0);
                    v.pitch = sound.pitch;
                }
            }
            None => {
                env.play_buffer_attached(
                    buffer,
                    handle,
                    position,
                    sound.volume.max(0.0),
                    sound.pitch,
                    voice_flags::LOOPING,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(kind: u32, socket: u16) -> Vec<u8> {
        let mut b = vec![0u8; 0x60];
        b[ABILITY_AT..ABILITY_AT + 4].copy_from_slice(&kind.to_le_bytes());
        b[SOCKET_AT..SOCKET_AT + 2].copy_from_slice(&socket.to_le_bytes());
        b
    }

    /// The shipped sword's section: type 3, one empty socket (0x80).
    #[test]
    fn the_shipped_flame_sword_is_cold_until_a_crystal_c_is_socketed() {
        let cold = ability_of(&body(3, 0x0080)).unwrap();
        assert_eq!(cold.kind, BURNING_BLADE);
        assert_eq!(check(Some(cold), AttachedRow::NONE), HeldFire::None);
        // The item GUI's drop: Power Crystal C is kind 4, its charge the
        // stack's bits 16–23.
        let mut sockets = exa_items::item_gui::Sockets([0x0080, 0, 0, 0]);
        sockets.drop_crystal(0, exa_items::possession::crystal::C, 0x12 << 16, false);
        assert_eq!(sockets.0[0], 0x1284);
        let lit = ability_of(&body(3, sockets.0[0])).unwrap();
        assert_eq!(check(Some(lit), AttachedRow::NONE), HeldFire::Ignite);
        // A live row is left burning; one on its way out is replaced.
        let live = AttachedRow {
            exists: true,
            extinguishing: false,
        };
        assert_eq!(check(Some(lit), live), HeldFire::None);
        let going = AttachedRow {
            exists: true,
            extinguishing: true,
        };
        assert_eq!(check(Some(lit), going), HeldFire::Ignite);
        // The crystal taken out (the socket's bare mark): the row goes out.
        assert_eq!(check(Some(cold), live), HeldFire::Extinguish);
        // A spent charge (254 and over) does not burn.
        let spent = ability_of(&body(3, 0xFE84)).unwrap();
        assert_eq!(check(Some(spent), AttachedRow::NONE), HeldFire::None);
    }

    /// Other crystals and other types light nothing.
    #[test]
    fn only_a_type_three_item_with_crystal_c_burns() {
        for socket in [0x0081, 0x0082, 0x0091] {
            let a = ability_of(&body(3, socket)).unwrap();
            assert_eq!(check(Some(a), AttachedRow::NONE), HeldFire::None);
        }
        for kind in [0, 1, 2, 4] {
            let a = ability_of(&body(kind, 0x0084)).unwrap();
            assert_eq!(check(Some(a), AttachedRow::NONE), HeldFire::None);
        }
        assert_eq!(check(None, AttachedRow::NONE), HeldFire::None);
        assert!(ability_of(&[0u8; 0x40]).is_none());
    }

    #[test]
    fn a_lit_type_three_blade_puts_the_player_light_out() {
        assert!(blade_puts_light_out([Some(3), None], true));
        assert!(!blade_puts_light_out([Some(3), None], false));
        assert!(!blade_puts_light_out([Some(1), None], true));
        assert_eq!(LIGHT_AT_CM, 72.5);
        assert_eq!(FLAME_TEXTURE, "flameletsa3");
    }
}
