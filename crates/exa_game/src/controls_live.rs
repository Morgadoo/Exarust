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
/// out-of-reach one and the left button's keep are the hand's here).
fn hand_bytes(control: Res<PlayerControl>, hand: Option<ResMut<crate::hand::Hand>>) {
    if control.is_changed()
        && control.off
        && let Some(mut hand) = hand
    {
        hand.out_of_reach = false;
        hand.keeps_left = false;
    }
}

/// The spirit form's switch on the frame's world
/// ([`PlayerControl::set_spirit_form`]): a window holds the controls with
/// only the powers' permission (on) or lets them go with all five (off),
/// and the hand's left button is kept off the world while it holds. On,
/// every game screen closes with the power bar (the GUI sweep with its bar
/// bit) and the dialogue ends. Its callers are the spirit form's start and
/// its way home ([`crate::mind::runtime`]).
pub fn spirit_controls(world: &mut World, on: bool) {
    if let Some(mut control) = world.get_resource_mut::<PlayerControl>() {
        control.set_spirit_form(on);
        info!(
            "controls: spirit form {} (hotkey mask {:#x})",
            if on { "holds them" } else { "lets them go" },
            control.hotkey_mask
        );
    }
    if let Some(mut hand) = world.get_resource_mut::<crate::hand::Hand>() {
        hand.keeps_left = on;
    }
    if on {
        crate::gui_screens::sweep_screens(world, crate::gui_screens::ScreenSweep::EveryAndBar);
        if world.contains_resource::<Messages<exa_script::LeaveDialogue>>() {
            world.write_message(exa_script::LeaveDialogue);
        }
    }
}

/// A pending end of the GUI's held left button: the death, the knock-out,
/// the level exit and a weapon coming loose from the player's hand ask the
/// window manager for a left-button release when its button word holds
/// the left button, which ends a held drag (a pack entry's, or the hand's
/// lift or drag) as a release there would.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EndGuiDrag(pub bool);

impl EndGuiDrag {
    /// Asks for the release (applied at the next frame's input).
    pub fn ask(world: &mut World) {
        if let Some(mut end) = world.get_resource_mut::<EndGuiDrag>() {
            end.0 = true;
        }
    }
}

/// The release itself, once the frame's input is read: a held left button
/// is released as if the window manager had seen it go up, so every
/// reader this frame sees the release. A button not held is left alone.
fn end_gui_drag(mut end: ResMut<EndGuiDrag>, buttons: Option<ResMut<ButtonInput<MouseButton>>>) {
    if !std::mem::take(&mut end.0) {
        return;
    }
    if let Some(mut buttons) = buttons
        && buttons.pressed(MouseButton::Left)
    {
        buttons.release(MouseButton::Left);
        info!("controls: the GUI's held left button is released");
    }
}

/// The controls' switch plugin.
pub struct ControlsLivePlugin;

impl Plugin for ControlsLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerControl>()
            .init_resource::<EndGuiDrag>()
            .add_systems(
                PreUpdate,
                (
                    character_mode,
                    end_gui_drag.after(bevy::input::InputSystems),
                ),
            )
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

    /// An asked release lets go of a held left button for the frame's
    /// readers; with the button up it does nothing.
    #[test]
    fn an_asked_release_ends_the_held_left_button() {
        let mut app = app();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        EndGuiDrag::ask(app.world_mut());
        app.update();
        let buttons = app.world().resource::<ButtonInput<MouseButton>>();
        assert!(!buttons.pressed(MouseButton::Left));
        assert!(buttons.just_released(MouseButton::Left));
        assert!(!app.world().resource::<EndGuiDrag>().0);
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
