//! The GUI reset every scene load runs, live: a level load, a save load, a
//! door's transition, an arena or practice level and the menu scene each
//! start with it ([`crate::save_scene::reset_game_gui`]).
//!
//! In the original's order: the death fade's reset gives a paused game its
//! time back ([`crate::fall::reset_death_fade`]); the sweep closes every
//! game screen and the power tree ([`crate::gui_screens::sweep_screens`],
//! the panels giving their icons back); the crystal drain forgets its
//! sources, which the next level's collect pass files again
//! ([`crate::crystal_drain`]); the item icon cache and its atlas are
//! emptied ([`crate::item_panel_draw::PanelIcons::clear`]). Exarust runs it
//! when a level change is asked for ([`LevelChangeRequest`]), every scene
//! load's way in. The game-main flag byte the original clears last has no
//! Exarust counterpart (logged).
//!
//! Evidence: the save unit's GUI reset and its four callees, read from the
//! instructions of `Exanima.exe`.

use bevy::prelude::*;
use exa_world::level_lifecycle::LevelChangeRequest;

use crate::save_scene::{GameGuiReset, reset_game_gui};

/// Runs the GUI reset on the frame's world.
pub fn apply_game_gui_reset(world: &mut World, reset: GameGuiReset) {
    if reset.death_fade && world.contains_resource::<exa_ui::DeathFade>() {
        world.resource_scope(|world, mut fade: Mut<exa_ui::DeathFade>| {
            let time = world.get_resource_mut::<Time<Virtual>>();
            crate::fall::reset_death_fade(&mut fade, time.map(Mut::into_inner));
        });
    }
    if reset.screens {
        crate::gui_screens::sweep_screens(world, crate::gui_screens::ScreenSweep::Every);
    }
    if reset.crystal_sources
        && let Some(mut drain) = world.get_resource_mut::<crate::crystal_drain::CrystalDrain>()
        && !drain.drain.sources.is_empty()
    {
        info!(
            "load reset: the crystal drain forgets {} source(s)",
            drain.drain.sources.len()
        );
        drain.drain.sources.clear();
    }
    if reset.icons {
        // The swept panels' handles go with the cache.
        if let Some(mut screens) = world.get_resource_mut::<crate::gui_screens::GameScreens>() {
            screens.released_icons.clear();
        }
        if let Some(mut icons) = world.get_resource_mut::<crate::item_panel_draw::PanelIcons>() {
            let held = icons.cache.entries.iter().filter(|e| e.id != 0).count();
            icons.clear();
            info!("load reset: the item icon cache is cleared ({held} icon(s) dropped)");
        }
    }
    debug!(
        "load reset: the game-main flag byte ({}) has no Exarust counterpart",
        reset.word
    );
}

/// A scene load's way in: each level change asked for runs the reset once.
fn reset_on_level_change(mut requests: MessageReader<LevelChangeRequest>, mut commands: Commands) {
    if requests.read().count() == 0 {
        return;
    }
    commands.queue(|world: &mut World| apply_game_gui_reset(world, reset_game_gui()));
}

/// The live GUI reset.
pub struct LoadResetPlugin;

impl Plugin for LoadResetPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LevelChangeRequest>().add_systems(
            Update,
            reset_on_level_change.before(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crystal_drain::CrystalDrain;
    use crate::gui_screens::GameScreens;
    use crate::item_panel_draw::PanelIcons;

    /// A level change runs the reset: the paused death fade ends and gives
    /// the time back, the crystal sources go, the icon cache and atlas are
    /// emptied and the swept panels' handles are forgotten.
    #[test]
    fn a_level_change_resets_the_fade_the_sources_and_the_icons() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(LoadResetPlugin)
            .insert_resource(exa_ui::DeathFade {
                started: Some(10.0),
                now: 20.0,
                paused: true,
            })
            .init_resource::<CrystalDrain>()
            .init_resource::<GameScreens>()
            .insert_resource(PanelIcons::new(4));
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        app.world_mut()
            .resource_mut::<CrystalDrain>()
            .drain
            .add(exa_script::objects::Source::Socketed(0x40));
        let (handle, _) = app
            .world_mut()
            .resource_mut::<PanelIcons>()
            .reserve(0x1234, 0, None);
        assert_eq!(handle, 1);
        app.world_mut()
            .resource_mut::<GameScreens>()
            .released_icons
            .push(handle);
        // No change asked: nothing is reset.
        app.update();
        assert!(app.world().resource::<exa_ui::DeathFade>().paused);
        app.world_mut()
            .write_message(LevelChangeRequest::change("exanima02", 0));
        app.update();
        let fade = app.world().resource::<exa_ui::DeathFade>();
        assert_eq!((fade.started, fade.paused), (None, false));
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        assert!(
            app.world()
                .resource::<CrystalDrain>()
                .drain
                .sources
                .is_empty()
        );
        let icons = app.world().resource::<PanelIcons>();
        assert!(icons.cache.entries.is_empty());
        assert!(icons.pending.is_empty());
        assert!(
            app.world()
                .resource::<GameScreens>()
                .released_icons
                .is_empty()
        );
        // The next icon takes the first slot again.
        let (again, _) = app
            .world_mut()
            .resource_mut::<PanelIcons>()
            .reserve(0x5678, 0, None);
        assert_eq!(again, 1);
    }
}
