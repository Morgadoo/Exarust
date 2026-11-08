//! The renderer's sector-major walk of the decal and blended lists.
//!
//! The original draws each render-state list sector by sector: the frame
//! walks the scene's sectors in scene order (the scene's own sector first,
//! then the rooms in the order they were appended), skips a sector whose
//! visibility word shares no bit with the renderer's mask, sets the
//! sector's scissor when it has one, and then walks the list's states in
//! list order, drawing that sector's objects of each state. The decal list
//! is drawn this way for every sector before the blended list is, and the
//! blended list again sector by sector (`docs/RENDERING.md`, Pass order).
//! So two blended surfaces in different rooms draw in room order, not in
//! state order across the whole scene.
//!
//! Bevy owns the transparent phase. [`crate::passes::order_bias`] already
//! puts each surface's state rank on its sort value (decals, then blended
//! ranks, then everything without a state by distance); this module adds
//! the sector: after Bevy's sort, the phase is re-sorted, stably, by
//! (list, sector), where the list is read back from the sort value and the
//! sector comes from [`DrawSectors`], which the live scene
//! (`exa_world::scene_live`) keeps for every mesh it registered in a room.
//! Within one sector and state Bevy's distance order stays. A scene with no
//! room sectors (every arena and outdoor level) leaves the phase alone.

use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::render::render_phase::{ViewSortedRenderPhases, sort_phase_system};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

use crate::passes::{MAX_RANKS, RANK_SPACING};

/// Every registered mesh outside the scene's own sector, by the position of
/// its sector in the scene's sector list (1 for the first room). Meshes not
/// listed are in the scene's own sector (0). `generation` changes whenever
/// the map does, so the render world copies it only then.
#[derive(Resource, Clone, Debug, Default)]
pub struct DrawSectors {
    pub map: EntityHashMap<u32>,
    pub generation: u64,
}

/// The render world's copy of [`DrawSectors`].
#[derive(Resource, Clone, Debug, Default)]
struct RenderDrawSectors {
    map: EntityHashMap<u32>,
    generation: u64,
}

pub struct SectorWalkPlugin;

impl Plugin for SectorWalkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawSectors>();
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<RenderDrawSectors>()
            .add_systems(ExtractSchedule, extract_sectors)
            .add_systems(
                Render,
                walk_sectors
                    .in_set(RenderSystems::PhaseSort)
                    .after(sort_phase_system::<Transparent3d>),
            );
    }
}

fn extract_sectors(main: Extract<Option<Res<DrawSectors>>>, mut render: ResMut<RenderDrawSectors>) {
    let Some(main) = main.as_deref() else {
        return;
    };
    if main.generation != render.generation {
        render.map.clone_from(&main.map);
        render.generation = main.generation;
    }
}

/// Which of the walk's lists a transparent-phase sort value belongs to:
/// 0 the decal list, 1 the blended list, 2 everything drawn after them
/// (no render state). The values are [`crate::passes::order_bias`]'s blocks
/// plus a view depth below one rank's spacing. A NaN goes last.
pub fn list_of_sort_value(value: f32) -> u8 {
    let block = (MAX_RANKS as f32 + 1.0) * RANK_SPACING;
    if value < -2.0 * block {
        0
    } else if value < -block {
        1
    } else {
        2
    }
}

/// The walk's key for one phase item: its list, and for the decal and
/// blended lists its sector (later lists are not walked by sector).
pub fn walk_key(value: f32, sector: u32) -> (u8, u32) {
    let list = list_of_sort_value(value);
    (list, if list < 2 { sector } else { 0 })
}

/// Re-sorts each view's transparent phase into the sector-major walk:
/// stably by [`walk_key`], so Bevy's order (state rank, then distance)
/// stays within one list and sector.
fn walk_sectors(
    sectors: Res<RenderDrawSectors>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    if sectors.map.is_empty() {
        return;
    }
    for phase in phases.values_mut() {
        let sector_of =
            |item: &Transparent3d| sectors.map.get(&item.entity.1.id()).copied().unwrap_or(0);
        let any = phase
            .items
            .values()
            .any(|item| list_of_sort_value(item.distance) < 2 && sector_of(item) != 0);
        if !any {
            continue;
        }
        phase
            .items
            .sort_by_cached_key(|_, item| walk_key(item.distance, sector_of(item)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::passes::order_bias;

    #[test]
    fn the_lists_read_back_from_the_bias() {
        assert_eq!(list_of_sort_value(order_bias(true, 0) + 3.0), 0);
        assert_eq!(list_of_sort_value(order_bias(true, MAX_RANKS) + 1000.0), 0);
        assert_eq!(list_of_sort_value(order_bias(false, 0)), 1);
        assert_eq!(list_of_sort_value(order_bias(false, MAX_RANKS) + 1000.0), 1);
        assert_eq!(list_of_sort_value(0.5), 2);
        assert_eq!(list_of_sort_value(-0.5), 2);
        assert_eq!(list_of_sort_value(f32::NAN), 2);
    }

    #[test]
    fn decals_then_blended_walk_by_sector() {
        // (sort value, sector) in Bevy's order.
        let items = [
            (order_bias(true, 0) + 5.0, 2),
            (order_bias(true, 1) + 1.0, 1),
            (order_bias(false, 0) + 9.0, 2),
            (order_bias(false, 0) + 2.0, 0),
            (order_bias(false, 3) + 1.0, 1),
            (4.0, 2),
            (7.0, 1),
        ];
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| walk_key(items[i].0, items[i].1));
        // Decals: sector 1 then 2; blended: sector 0, 1, 2; the rest by
        // distance, sectors ignored.
        assert_eq!(order, vec![1, 0, 3, 4, 2, 5, 6]);
    }
}
