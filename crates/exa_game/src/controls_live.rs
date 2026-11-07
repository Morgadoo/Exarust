//! The player controls' off switch, live ([`PlayerControl::set_off`]).
//!
//! The singleton is created with its controls off. The Character mode's
//! start (the first frame a player exists) turns them on and its end (the
//! player gone) turns them off; the Spectate mode's start and end do the
//! same ([`crate::spectate`]). A form that suspends the controls turns them
//! off when it opens and on when it closes: the skill screen
//! ([`crate::skill_screen`]), the power tree ([`crate::power_screen`]) and
//! the arena arsenal ([`crate::arsenal_view`]). The inventory, the
//! character view and the menus leave them alone.
//!
//! While they are off the player's input does not reach the game:
//! - the key-down handler and the pad routine dispatch nothing
//!   ([`crate::hotkeys`]), and the power tree's key does not open it;
//! - the movement request and the behaviour read no key, no mouse button
//!   and no pad; the switch clears the move request and the locomotion
//!   flags once ([`crate::player`]'s steering);
//! - the mouse capture for the orbit and Cursor Look is dropped and not
//!   started ([`crate::player`]'s cursor look);
//! - the idle camera does not run ([`crate::idle_orbit`]);
//! - the attack button is not latched, and a held wake is not released;
//! - the hand's out-of-reach byte is cleared.
//!
//! The skill screen and the power tree are forms over the whole window,
//! so the pointer is over the GUI while they are open
//! ([`crate::panel_live::PointerOverGui`]) and the world's press and
//! click paths leave it to them, as the original's window manager gives
//! their forms the mouse before the game view.
//!
//! Evidence: the controls' switch routine and its ten callers (the two
//! modes' start and end hooks, the skill unit's open and close, the power
//! tree's open and close, the arsenal's open and close), the key-down
//! handler's and the frame task's off tests, read in the instructions.

use bevy::prelude::*;

use crate::player::Player;
use crate::player_controls::PlayerControl;

/// Turns the controls on or off and logs the change, with what asked.
pub fn switch_controls(control: &mut PlayerControl, off: bool, by: &str) {
    if control.set_off(off) {
        info!("controls: {} ({by})", if off { "off" } else { "on" });
    }
}

/// The Character mode's end and start: the player gone turns the controls
/// off, a new player turns them on. Runs before the frame's fixed steps,
/// so the new player's first behaviour tick already reads them on.
fn character_mode(
    mut control: ResMut<PlayerControl>,
    mut removed: RemovedComponents<Player>,
    added: Query<(), Added<Player>>,
    players: Query<(), With<Player>>,
    spectate: Option<Res<crate::spectate::SpectateSession>>,
) {
    // A player taken away for the Spectate mode has had its end run by the
    // mode switch already.
    let spectating = crate::spectate::SpectateSession::spectating(spectate.as_deref());
    if removed.read().next().is_some() && players.is_empty() && !spectating {
        switch_controls(&mut control, true, "end of play");
    }
    if !added.is_empty() {
        switch_controls(&mut control, false, "start of play");
    }
}

/// The switch's effect on the hand: off clears its per-frame bytes (the
/// out-of-reach one is the hand's here).
fn hand_bytes(control: Res<PlayerControl>, hand: Option<ResMut<crate::hand::Hand>>) {
    if control.is_changed()
        && control.off
        && let Some(mut hand) = hand
    {
        hand.out_of_reach = false;
    }
}

/// The controls' switch plugin.
pub struct ControlsLivePlugin;

impl Plugin for ControlsLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerControl>()
            .add_systems(PreUpdate, character_mode)
            .add_systems(Update, hand_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(ControlsLivePlugin);
        app
    }

    /// The controls start off, a player's arrival turns them on and its
    /// removal off again.
    #[test]
    fn the_character_mode_turns_the_controls_on_and_off() {
        let mut app = app();
        app.update();
        assert!(app.world().resource::<PlayerControl>().off);
        let player = app.world_mut().spawn(Player).id();
        app.update();
        assert!(!app.world().resource::<PlayerControl>().off);
        app.world_mut().despawn(player);
        app.update();
        assert!(app.world().resource::<PlayerControl>().off);
    }

    /// A player rebuilt in the same frame keeps the controls on.
    #[test]
    fn a_rebuilt_player_keeps_them_on() {
        let mut app = app();
        let player = app.world_mut().spawn(Player).id();
        app.update();
        app.world_mut().despawn(player);
        app.world_mut().spawn(Player);
        app.update();
        assert!(!app.world().resource::<PlayerControl>().off);
    }
}
