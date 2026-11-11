//! The terrain's shadow draws: which 6 m blocks cast into the sun's map.
//!
//! The original's two terrain shadow draws (all of a tile's blocks, or one
//! block for a light's cube face) skip a block whose node has the
//! external-draw bit, and draw only a block whose visible word has its
//! main-view bit: the bit the scene's visible pass sets on a node it drew
//! this frame. A block the camera does not see casts no terrain shadow.
//!
//! Bevy's sun pass instead casts every block inside a cascade's frustum.
//! [`sun_casts_drawn_blocks`] runs after Bevy's light visibility check and
//! takes out of the sun's cascade lists the blocks the live scene's visible
//! pass left undrawn ([`crate::scene_live::LiveScene::drawn_by_main`]). A
//! block the scene does not hold (no live scene, or not yet registered) is
//! kept. `EXA_TERRAIN_SHADOW_ALL=1` turns the rule off, to compare.
//!
//! The cube faces of point lights (`exa_render::cube_shadows`) take the
//! same rule: [`cube_faces_skip_undrawn_blocks`] lists the blocks the visible
//! pass left undrawn in [`CubeCasterGate`] before the faces are built, and a
//! face rendered this frame leaves them out. The faces stay cached as
//! rendered, as the original's do: its one-block draw tests the bit when the
//! face renders and nothing dirties a face when a block's bit changes, so a
//! face rendered while a block was out of view lacks its shadow until a
//! caster or the light dirties the face again.
//! Rules: `docs/RENDERING.md` (Terrain shadows).

use bevy::camera::visibility::CascadesVisibleEntities;
use bevy::prelude::*;
use exa_render::cube_shadows::CubeCasterGate;

use super::TerrainBlock;
use crate::scene_live::LiveScene;

/// Whether the sun draws only the blocks the main view drew (the original's
/// rule; `EXA_TERRAIN_SHADOW_ALL=1` turns it off).
#[derive(Resource, Clone, Copy, Debug)]
pub struct TerrainShadowRule {
    pub drawn_only: bool,
}

impl Default for TerrainShadowRule {
    fn default() -> Self {
        Self {
            drawn_only: !std::env::var("EXA_TERRAIN_SHADOW_ALL").is_ok_and(|v| v == "1"),
        }
    }
}

/// The last frame's sun cascade entries of terrain blocks: kept and taken
/// out, over every view and cascade.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct TerrainShadowStats {
    pub kept: usize,
    pub left_out: usize,
    frames: u64,
}

/// Whether a terrain block casts into the sun's map or a cube face rendered
/// this frame: a block the scene holds casts only when its visible pass
/// drew it; one it does not hold keeps Bevy's answer.
pub fn casts_into_sun(drawn_by_main: Option<bool>) -> bool {
    drawn_by_main != Some(false)
}

/// Takes the terrain blocks the main view did not draw out of every sun
/// cascade's caster list.
pub fn sun_casts_drawn_blocks(
    rule: Res<TerrainShadowRule>,
    live: Option<Res<LiveScene>>,
    blocks: Query<(), With<TerrainBlock>>,
    mut suns: Query<&mut CascadesVisibleEntities>,
    mut stats: ResMut<TerrainShadowStats>,
) {
    let Some(live) = live.filter(|l| rule.drawn_only && l.scene.is_some()) else {
        return;
    };
    let (mut kept, mut left_out) = (0, 0);
    for mut cascades in &mut suns {
        for lists in cascades.entities.values_mut() {
            for list in lists.iter_mut() {
                list.entities.retain(|&e| {
                    if !blocks.contains(e) {
                        return true;
                    }
                    let casts = casts_into_sun(live.drawn_by_main(e));
                    if casts {
                        kept += 1;
                    } else {
                        left_out += 1;
                    }
                    casts
                });
            }
        }
    }
    stats.kept = kept;
    stats.left_out = left_out;
    stats.frames += 1;
    if stats.frames % 240 == 1 && kept + left_out > 0 {
        info!(
            "terrain shadow draw: the sun casts {kept} block entries the main view drew and leaves out {left_out}"
        );
    }
}

/// Lists the terrain blocks the main view did not draw for this frame's
/// cube faces to leave out (the original's one-block shadow draw's test of
/// the visible word). Runs after the scene's visible pass and before the
/// faces are built.
pub fn cube_faces_skip_undrawn_blocks(
    rule: Res<TerrainShadowRule>,
    live: Option<Res<LiveScene>>,
    blocks: Query<Entity, With<TerrainBlock>>,
    gate: Option<ResMut<CubeCasterGate>>,
    cube: Option<Res<exa_render::cube_shadows::CubeStats>>,
    mut frames: Local<u64>,
) {
    let Some(mut gate) = gate else { return };
    gate.left_out.clear();
    let Some(live) = live.filter(|l| rule.drawn_only && l.scene.is_some()) else {
        return;
    };
    gate.left_out.extend(
        blocks
            .iter()
            .filter(|&e| !casts_into_sun(live.drawn_by_main(e))),
    );
    // The faces' count is the previous frame's (they are built later).
    let gated = cube.map_or(0, |c| c.gated);
    *frames += 1;
    if *frames % 240 == 1 && gated > 0 {
        info!(
            "terrain shadow draw: the cube faces left out {gated} undrawn block draws ({} blocks undrawn)",
            gate.left_out.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_held_undrawn_block_is_left_out() {
        assert!(casts_into_sun(Some(true)));
        assert!(!casts_into_sun(Some(false)));
        assert!(casts_into_sun(None));
    }
}
