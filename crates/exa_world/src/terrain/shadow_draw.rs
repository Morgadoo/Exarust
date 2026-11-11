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
//! The cached cube faces of point lights (`exa_render::cube_shadows`) keep
//! every linked block: a face rendered while a block was out of view would
//! keep its missing shadow until the face is dirtied again.
//! Rules: `docs/RENDERING.md` (Terrain shadows).

use bevy::camera::visibility::CascadesVisibleEntities;
use bevy::prelude::*;

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

/// Whether a terrain block casts into the sun's map: a block the scene
/// holds casts only when its visible pass drew it; one it does not hold
/// keeps Bevy's answer.
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
