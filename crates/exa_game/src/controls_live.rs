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
//! The block pair is separate ([`PlayerControl::set_blocked`]): the
//! knock-out, the death, the practice ladder's last win and the arena
//! manager's fall block the controls through the flow's
//! [`crate::flow::Step::Block`] and the arena match tick; the revive
//! releases them and the start of play clears the byte. While blocked the
//! input is suspended as while off, the player's behaviour stops after
//! its head, the left press picks nothing and the idle camera turns ten
//! seconds on; the hotkeys still run. The second block is the arena
//! loadout form's ([`loadout_form_block`]): it also refuses the hotkeys.
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

/// The block routine's pair ([`PlayerControl::set_blocked`]), logged with
/// what asked. The pair has no change test of its own; the log does.
pub fn block_controls(control: &mut PlayerControl, on: bool, by: &str) {
    if control.set_blocked(on) {
        info!(
            "controls: {} ({by})",
            if on { "blocked" } else { "unblocked" }
        );
    }
}

/// The second block's pair ([`PlayerControl::set_second_block`]), logged.
pub fn second_block_controls(control: &mut PlayerControl, on: bool, by: &str) {
    if control.set_second_block(on) {
        info!(
            "controls: second block {} ({by})",
            if on { "on" } else { "off" }
        );
    }
}

/// The Character mode's end and start: the player gone turns the controls
/// off, a new player turns them on. Runs before the frame's fixed steps,
/// so the new player's first behaviour tick already reads them on. The
/// start of play also clears the block byte (directly, not through the
/// block pair).
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
        block_controls(&mut control, false, "start of play");
        switch_controls(&mut control, false, "start of play");
    }
}

/// The arena's loadout form holds the controls with the second block
/// while it is open: its show sets it, its close releases it (both only
/// with the hub's world, which [`crate::hub_windows::HubWindows`] stands
/// for). The recruit's creator does the same in the original; the hub
/// does not open it.
fn loadout_form_block(
    mut control: ResMut<PlayerControl>,
    windows: Option<Res<crate::hub_windows::HubWindows>>,
    mut was_open: Local<bool>,
) {
    let open =
        windows.is_some_and(|w| w.forms.get(crate::hub_forms::FormWindow::Loadout).is_some());
    if open != *was_open {
        *was_open = open;
        let by = if open {
            "loadout form opens"
        } else {
            "loadout form closes"
        };
        second_block_controls(&mut control, open, by);
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
            .add_systems(Update, (hand_bytes, loadout_form_block));
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

    /// The start of play clears the block byte, not the second block.
    #[test]
    fn the_start_of_play_clears_the_block() {
        let mut app = app();
        app.update();
        {
            let mut control = app.world_mut().resource_mut::<PlayerControl>();
            control.set_blocked(true);
            control.set_second_block(true);
        }
        app.world_mut().spawn(Player);
        app.update();
        let control = app.world().resource::<PlayerControl>();
        assert!(!control.blocked);
        assert!(control.second_block);
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
