//! The player's hotkeys: what a key press does in play, as the player
//! controls' key handler decides it. Rules: `docs/PLAYER.md` (Hotkeys).
//!
//! A pressed key's low byte is compared with the bound keys. A dead player
//! only ends a dialogue. With the power bar open, a digit picks a power
//! ('0' the tenth), Q uses the picked one, and the arrows step through
//! them; with a dialogue open, '1'–'9' choose an answer and the Talk key
//! ends it; with neither, the Talk key talks, if allowed. Any other key goes
//! on, in this order: Q uses the picked power (if powers are allowed), Tab
//! toggles combat mode, I opens the inventory (if allowed, and in the
//! modes that hide it only while collapsing), R asks for the weapon sets to
//! swap (if allowed), F lights or puts out the held light, K the skills and
//! J the journal (if allowed and not hidden), P the power tree (if powers
//! are allowed and the player has some) and C the power bar (when the
//! player or the companion has powers). The permissions are a mask the
//! game sets: all five (skills and journal 1, inventory 2, powers 4, talk
//! 8, swap 0x10) in play.
//!
//! Wired in Exarust: Tab (with the hand's cancel and the request bits it
//! clears), R (with the player's condition word and the lift counting as a
//! drag) and F (the record's light flag, then a flip request to the grip
//! runtime, as the original starts the held item's flip in the same
//! branch); I and J send a
//! [`crate::gui_screens::ScreenRequest`], which opens the inventory and the
//! journal on the window manager; C, the digits, Q and the arrows send the
//! power bar's commands to [`crate::thaum::cast`]; Talk, the digits and
//! the leave key send the conversation engine's messages, Talk to the
//! character [`crate::dialogue_live`] found. K opens the skill screen
//! ([`crate::skill_screen`]) and the power tree has its own key reader.
//!
//! The pad's newly pressed commands go through [`pad_hotkeys`], the
//! controls' per-frame pad routine: Powers, UsePower and the bar's steps,
//! PowerTree, Combat (the combat-mode toggle), SwapWeapon, Inventory and
//! Skills, with its own gates (no
//! death test, the bar only with the player's own powers, Skills ungated).
//!
//! Evidence: the controls' key handler, the combat toggle and the swap and
//! light requests, read in the instructions
//! (`context/re/player-control-hand.md`, Hotkeys).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::player_controls::PlayerControl;
use exa_character::grip_runtime::FlipWeaponRequest;
use exa_character::held::Hands;
use exa_combat::Condition;
use exa_items::possession::{HeldMark, Possessions, SwapRequest};
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::{Locomotion, flags};

use exa_ui::game_gui::GameScreen;

use crate::bindings::{Action, Bindings, virtual_key};
use crate::gui_screens::ScreenRequest;
use crate::hand::{Hand, Holding};
use crate::player::PlayerBody;
use crate::thaum::cast::{BarAction, BarCommand, PowerCaster};

/// The character the Talk key talks to, by character id. The talk
/// routine's target query fills it on the frame the key is pressed
/// ([`crate::dialogue_live`]): the nearby character whose chest shows
/// nearest the cursor.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DialogueFocus(pub Option<u32>);

/// What a dialogue key asks the conversation engine to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueCommand {
    /// Start talking to this character.
    Talk(u32),
    /// Pick panel entry `n` (from zero).
    Pick(usize),
    /// End every conversation the player is in.
    Leave,
}

/// The command a hotkey sends. The Talk key ends a running dialogue (it is
/// [`Hotkey::EndDialogue`] then) and otherwise talks to the focused
/// character, if there is one; the digits pick an entry.
pub fn dialogue_command(action: Hotkey, focus: Option<u32>) -> Option<DialogueCommand> {
    match action {
        Hotkey::EndDialogue => Some(DialogueCommand::Leave),
        Hotkey::Choose(n) => Some(DialogueCommand::Pick(usize::from(n))),
        Hotkey::Talk => focus.map(DialogueCommand::Talk),
        _ => None,
    }
}

/// The dialogue state and messages the key handler reads and writes.
#[derive(SystemParam)]
pub struct DialogueSide<'w> {
    conversations: Option<Res<'w, exa_script::Conversations>>,
    focus: Option<Res<'w, DialogueFocus>>,
    talk: MessageWriter<'w, exa_script::Talk>,
    pick: MessageWriter<'w, exa_script::PickReply>,
    leave: MessageWriter<'w, exa_script::LeaveDialogue>,
}

/// The permission bits of the key handler's mask.
pub mod allow {
    pub const SKILLS: u32 = 0x1;
    pub const INVENTORY: u32 = 0x2;
    pub const POWERS: u32 = 0x4;
    pub const TALK: u32 = 0x8;
    pub const SWAP: u32 = 0x10;
    /// The mask in play.
    pub const PLAY: u32 = 0x1F;
}

/// The request bits entering combat mode clears: the backpedal, the keys'
/// heading and the turn-about.
pub const COMBAT_CLEARS: u32 = flags::BACK | flags::KEYS | flags::TURN_ABOUT;

/// What a hotkey does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    EndDialogue,
    /// Choose the dialogue answer (from 0).
    Choose(u8),
    /// Pick a power of the bar (from 0), or use the picked one (−1).
    Power(i8),
    /// Step through the bar's powers.
    StepPower(i8),
    Talk,
    ToggleCombat,
    Inventory,
    SwapSets,
    ToggleLight,
    Skills,
    Journal,
    PowerTree,
    PowerBar,
    /// The pad's Select: the end-of-session menu confirms its row.
    ConfirmEndMenu,
}

/// What the key handler reads besides the key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Context {
    pub dead: bool,
    pub dialogue: bool,
    pub power_bar: bool,
    /// The permission mask ([`allow`]).
    pub allowed: u32,
    /// A mode that hides the inventory (opened only while collapsing).
    pub hides_inventory: bool,
    pub collapsing: bool,
    /// A mode that hides the skills and the journal.
    pub hides_journal: bool,
    /// How many powers the player and the companion have.
    pub powers: i32,
    pub companion_powers: Option<i32>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            dead: false,
            dialogue: false,
            power_bar: false,
            allowed: allow::PLAY,
            hides_inventory: false,
            collapsing: false,
            hides_journal: false,
            powers: 0,
            companion_powers: None,
        }
    }
}

/// What the key with virtual-key code `key` does, the keys bound as in
/// `bindings`.
pub fn hotkey(key: u32, bindings: &Bindings, c: Context) -> Option<Hotkey> {
    if c.dead {
        return c.dialogue.then_some(Hotkey::EndDialogue);
    }
    let byte = (key & 0xFF) as u8;
    let is = |action: Action| u32::from(byte) == bindings.key(action) & 0xFFFF;
    let allowed = |bit: u32| c.allowed & bit != 0;
    if c.power_bar {
        if (0x30..=0x39).contains(&byte) {
            return Some(Hotkey::Power(if byte == 0x30 {
                9
            } else {
                (byte - 0x31) as i8
            }));
        }
        if is(Action::UsePower) {
            return Some(Hotkey::Power(-1));
        }
        if is(Action::PowerNext) {
            return Some(Hotkey::StepPower(1));
        }
        if is(Action::PowerPrevious) {
            return Some(Hotkey::StepPower(-1));
        }
    } else if c.dialogue {
        if (0x31..=0x39).contains(&byte) {
            return Some(Hotkey::Choose(byte - 0x31));
        }
        if is(Action::Talk) {
            return Some(Hotkey::EndDialogue);
        }
    } else if is(Action::Talk) {
        return allowed(allow::TALK).then_some(Hotkey::Talk);
    }
    if is(Action::UsePower) {
        return allowed(allow::POWERS).then_some(Hotkey::Power(-1));
    }
    if is(Action::ToggleCombat) {
        Some(Hotkey::ToggleCombat)
    } else if is(Action::Inventory) {
        (allowed(allow::INVENTORY) && (!c.hides_inventory || c.collapsing))
            .then_some(Hotkey::Inventory)
    } else if is(Action::SwitchEquipped) {
        allowed(allow::SWAP).then_some(Hotkey::SwapSets)
    } else if is(Action::FlipWeapon) {
        Some(Hotkey::ToggleLight)
    } else if is(Action::Skills) {
        (allowed(allow::SKILLS) && !c.hides_journal).then_some(Hotkey::Skills)
    } else if is(Action::Journal) {
        (allowed(allow::SKILLS) && !c.hides_journal).then_some(Hotkey::Journal)
    } else if is(Action::PowerTree) {
        (allowed(allow::POWERS) && c.powers > 0).then_some(Hotkey::PowerTree)
    } else if is(Action::PowerBar) {
        let has = if c.powers < 1 {
            c.companion_powers.is_some_and(|n| n > 0)
        } else {
            true
        };
        has.then_some(Hotkey::PowerBar)
    } else {
        None
    }
}

/// What the pad's newly pressed command bits ask of the controls this
/// frame, as the controls' per-frame pad routine decides it (the keyboard's
/// counterpart is [`hotkey`]), in its order:
///
/// - Powers (`POWER_BAR`) toggles the bar when the player has a power (the
///   player's own: the key also counts the companion's);
/// - with the bar closed, UsePower uses the picked power when powers are
///   allowed; with it open, UsePower uses it whatever the mask, else a
///   step right moves the pick on and else a step left moves it back;
/// - PowerTree (or Powers held 500 ms) opens the tree when powers are
///   allowed and the player has one;
/// - Combat toggles combat mode;
/// - SwapWeapon asks for the swap when allowed;
/// - Inventory opens it when allowed and, in a mode that hides it, only
///   while collapsing;
/// - Skills asks for the skill screen, with no permission test.
///
/// Unlike the key handler the routine does not test death. Talk, Select
/// and the dialogue's steps are the conversation's ([`crate::dialogue_live`]),
/// and the pause menu the menu's ([`crate::menu_screen`]). The pad's attack
/// is not a command bit: the original's pad attacks through the Steam
/// controller layer's emulated mouse button
/// ([`crate::gamepad::EMULATED_LEFT_BUTTON`]).
pub fn pad_hotkeys(bits: u32, c: Context) -> Vec<Hotkey> {
    use crate::gamepad::command;
    let allowed = |bit: u32| c.allowed & bit != 0;
    let mut out = Vec::new();
    if bits & command::POWER_BAR != 0 && c.powers > 0 {
        out.push(Hotkey::PowerBar);
    }
    // Select outside a dialogue confirms the end menu's row (the menu's
    // own test: only while it shows).
    if bits & command::USE != 0 && !c.dialogue {
        out.push(Hotkey::ConfirmEndMenu);
    }
    if !c.power_bar {
        if bits & command::USE_POWER != 0 && allowed(allow::POWERS) {
            out.push(Hotkey::Power(-1));
        }
    } else if bits & command::USE_POWER != 0 {
        out.push(Hotkey::Power(-1));
    } else if bits & command::STEP_RIGHT != 0 {
        out.push(Hotkey::StepPower(1));
    } else if bits & command::STEP_LEFT != 0 {
        out.push(Hotkey::StepPower(-1));
    }
    if bits & command::POWER_TREE != 0 && allowed(allow::POWERS) && c.powers > 0 {
        out.push(Hotkey::PowerTree);
    }
    if bits & command::COMBAT != 0 {
        out.push(Hotkey::ToggleCombat);
    }
    if bits & command::SWAP_WEAPON != 0 && allowed(allow::SWAP) {
        out.push(Hotkey::SwapSets);
    }
    if bits & command::INVENTORY != 0
        && allowed(allow::INVENTORY)
        && (!c.hides_inventory || c.collapsing)
    {
        out.push(Hotkey::Inventory);
    }
    if bits & command::SKILLS != 0 {
        out.push(Hotkey::Skills);
    }
    out
}

/// Entering combat mode: the hand lets go and the request's backpedal,
/// keys' heading and turn-about bits clear. A lifted item is cancelled:
/// the hold frame puts it back where its lift saved it
/// ([`Hand::cancel_lift`]), as the original's lift cancel restores the
/// saved frames. The original's cancel leaves a drag's link in place;
/// Exarust lets go of the drag (the answer is the dragged body).
pub fn enter_combat(hand: &mut Hand, req: &mut Locomotion) -> Option<Entity> {
    req.flags &= !COMBAT_CLEARS;
    let held = match hand.holding {
        Holding::Drag { body, .. } => Some(body),
        Holding::Lift => {
            hand.cancel_lift = true;
            None
        }
        _ => None,
    };
    hand.holding = Holding::Nothing;
    held
}

pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut App) {
        exa_physics::sim_app::add_fixed_commands(app);
        app.add_message::<ScreenRequest>()
            .add_message::<crate::end_menu_screen::EndMenuConfirm>()
            .add_message::<BarCommand>()
            .add_message::<FlipWeaponRequest>()
            .add_message::<crate::skill_screen::OpenSkills>()
            .add_message::<exa_script::Talk>()
            .add_message::<exa_script::PickReply>()
            .add_message::<exa_script::LeaveDialogue>()
            .init_resource::<DialogueFocus>()
            .add_systems(
                Update,
                (
                    retry_power_hold,
                    press_hotkeys
                        .after(crate::dialogue_live::pad_talk)
                        .after(crate::item_card::tick_card)
                        .after(crate::dialogue_live::panel_input),
                )
                    .chain()
                    .in_set(HotkeySet)
                    .after(crate::player::cursor_ground)
                    .before(crate::player::player_combat)
                    .in_set(exa_physics::FramePhase::Gui),
            );
    }
}

/// The system that reads the player's hotkeys.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HotkeySet;

/// The head of the controls' per-frame pad routine: the power bar's hold
/// on a slot whose power answered "not now" retries the cast every frame
/// ([`crate::player_controls::PowerHold::frame`]); past its 500 ms it lets
/// go and asks once for the picked power. A retry answered with anything
/// but "not now" lets the hold go, and "not now" renews it
/// ([`BarAction::Retry`]). The routine runs while the controls are on
/// (blocked or not); the clock is the powers' tick clock.
fn retry_power_hold(
    control: Option<ResMut<PlayerControl>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    players: Query<Entity, (With<PlayerBody>, With<PowerCaster>)>,
    mut bars: MessageWriter<BarCommand>,
) {
    let Some(mut control) = control else {
        return;
    };
    if control.off || control.power_hold.slot == 0 {
        return;
    }
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let Some(slot) = control.power_hold.frame(now) else {
        return;
    };
    let Some(caster) = players.iter().next() else {
        return;
    };
    debug!("power hold: retry (slot {slot})");
    bars.write(BarCommand {
        caster,
        action: BarAction::Retry,
    });
}

/// Releases the hand's grab on `body` (run at the start of a fixed step).
fn release_grab(In(body): In<Entity>, mut bodies: Query<&mut Ragdoll, Without<PlayerBody>>) {
    if let Ok(mut ragdoll) = bodies.get_mut(body) {
        ragdoll.release_grab();
    }
}

/// The frame's hotkeys for the player.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn press_hotkeys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    hand: Option<ResMut<Hand>>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    mut players: Query<(
        Entity,
        &mut PlayerBody,
        &mut Locomotion,
        Option<&Condition>,
        Option<&Hands>,
        Option<&mut Possessions>,
        Option<&HeldMark>,
        Option<&mut exa_anim::Animator>,
    )>,
    casters: Query<&PowerCaster>,
    mut fixed: ResMut<exa_physics::sim_app::FixedCommands>,
    mut screens: MessageWriter<ScreenRequest>,
    mut bars: MessageWriter<BarCommand>,
    mut flips: MessageWriter<FlipWeaponRequest>,
    mut skills: MessageWriter<crate::skill_screen::OpenSkills>,
    (mut dialogue, control, mut confirms, gui, hub, panel_drag): (
        DialogueSide,
        Option<Res<PlayerControl>>,
        MessageWriter<crate::end_menu_screen::EndMenuConfirm>,
        Option<Res<crate::gui_screens::GameScreens>>,
        Option<Res<crate::hub_windows::HubWindows>>,
        Option<Res<crate::panel_live::PanelDrag>>,
    ),
) {
    // The controls off, the key-down handler and the pad routine (the
    // controls' frame task) dispatch nothing; the second block refuses the
    // keys too (the block alone does not).
    if PlayerControl::keys_refused(control.as_deref()) {
        return;
    }
    // The window manager hands the keys to its focused form: with the
    // journal's text box focused they type there and never reach the
    // viewport's key handler.
    if gui.as_deref().is_some_and(|gui| gui.journal_box_focused()) {
        return;
    }
    // The key handler is the game view's: a key-down reaches it only while
    // the view has the window manager's focus. With a game screen under the
    // pointer the key goes to that form's key slot and climbs to the task
    // instead (`gui_screens::screens_keys`). The pad's commands are the
    // controls' own frame routine and are not gated.
    let to_window = gui.as_deref().is_some_and(|gui| gui.keys_to_window());
    let pressed: Vec<u32> = keys
        .get_just_pressed()
        .filter(|_| !to_window)
        .filter_map(|&k| virtual_key(k))
        .collect();
    let pad_frame = pad_status
        .as_ref()
        .is_some_and(|s| s.active())
        .then(|| pad.as_ref().map(|p| p.frame()))
        .flatten();
    // The pad's other commands, as the controls' per-frame pad routine
    // reads them ([`pad_hotkeys`]).
    let pad_bits = pad_frame.map_or(0, |frame| frame.pressed);
    if pressed.is_empty() && pad_bits == 0 {
        return;
    }
    let mut hand = hand;
    for (entity, mut pb, mut req, condition, hands, mut possessions, mark, mut animator) in
        &mut players
    {
        let word = condition.copied().unwrap_or_default();
        let caster = casters.get(entity).ok();
        let context = Context {
            dead: word.is_dead(),
            collapsing: word.dying(),
            dialogue: dialogue
                .conversations
                .as_deref()
                .is_some_and(|c| c.engine.panel.open),
            power_bar: caster.is_some_and(|c| c.bar_open),
            powers: caster.map_or(0, |c| c.known.len() as i32),
            // The controls' permission mask (the spirit form keeps only
            // the powers'), and the byte a holding window sets: the
            // spirit form's, or the shop's trade mode.
            allowed: PlayerControl::hotkey_mask(control.as_deref()),
            hides_inventory: PlayerControl::window_holds(control.as_deref())
                || hub
                    .as_deref()
                    .and_then(|h| h.shop.as_ref())
                    .is_some_and(|shop| shop.window.trading),
            ..Context::default()
        };
        let keyed = pressed.iter().filter_map(|&key| {
            let action = hotkey(key, &bindings, context)?;
            debug!("hotkey {key:#x}: {action:?}");
            Some(action)
        });
        let padded = pad_hotkeys(pad_bits, context);
        if !padded.is_empty() {
            info!("pad commands {pad_bits:#x}: {padded:?}");
        }
        let actions: Vec<Hotkey> = keyed.chain(padded).collect();
        for action in actions {
            match action {
                Hotkey::ToggleCombat => {
                    if pb.toggle_combat() {
                        req.entered_combat();
                        // Entering combat mode closes every in-game screen
                        // and the power tree (the bar stays).
                        commands.queue(|world: &mut World| {
                            crate::gui_screens::sweep_screens(
                                world,
                                crate::gui_screens::ScreenSweep::Every,
                            );
                        });
                    }
                    pb.aim = req.heading;
                    if pb.combat
                        && let Some(hand) = hand.as_deref_mut()
                        && let Some(body) = enter_combat(hand, &mut req)
                    {
                        // The grab goes at the start of the next fixed step
                        // (a write to the held body).
                        fixed.run(release_grab, body);
                    }
                }
                Hotkey::SwapSets => {
                    // The gate's drag is the GUI's (an item drag in
                    // flight); a lift in the hand refuses too [Exarust
                    // guard]. The body's own open inventory swaps at once;
                    // otherwise the request writes the switch motion into
                    // the body's requested motion: the body tick starts it
                    // when the body is free, and the clip's swap point
                    // swaps the sets (`equip::swap_at_switch_point`).
                    let lifting = hand.as_deref().is_some_and(|h| h.holding == Holding::Lift);
                    let dragging = panel_drag.as_deref().is_some_and(|d| d.source.is_some());
                    let pack_open = gui.as_deref().is_some_and(|gui| {
                        gui.find(exa_ui::game_gui::GameScreen::Inventory).is_some()
                            && gui.inventory_bound(Some(entity)) == Some(entity)
                    });
                    let answer = Possessions::swap_request(word.0, lifting || dragging, pack_open);
                    debug!("swap sets: {answer:?}");
                    match answer {
                        SwapRequest::Refused => {}
                        SwapRequest::Now => {
                            commands
                                .entity(entity)
                                .insert(exa_items::catalogue::SwapRequested);
                        }
                        SwapRequest::Animate => {
                            if let Some(animator) = animator.as_mut() {
                                animator.requested =
                                    exa_combat::condition::action::SWITCH_WEAPONS as i32;
                            }
                        }
                    }
                }
                Hotkey::ToggleLight => {
                    let first =
                        hands.and_then(|h| h.main.map(|m| m.light).or(h.off_light.then_some(true)));
                    if let Some(p) = possessions.as_deref_mut() {
                        // The original reads the held part's own mark, not
                        // the record's bit (a doll drop keeps only the top
                        // bit of a lit item's flags in the record).
                        let burning = p.burning(mark);
                        // The original's branch ends by starting the held
                        // item's flip, whose end turns the part's mark over.
                        if p.toggle_light(
                            first.is_some(),
                            word.0 & Condition::SWITCHING.0 != 0,
                            first.unwrap_or(false),
                            burning,
                        ) {
                            commands.entity(entity).insert(HeldMark {
                                item: p.first_held(),
                                lit: !burning,
                            });
                            info!(
                                "light toggle: {:#x} was {}, record hand flags now {:#x}",
                                p.first_held(),
                                if burning { "burning" } else { "out" },
                                p.out[0].flags
                            );
                            flips.write(FlipWeaponRequest { body: entity });
                        }
                    }
                }
                Hotkey::Inventory | Hotkey::Journal => {
                    // The journal's opener leaves combat mode first, when
                    // the player is in action and alive, whether it then
                    // opens, focuses or closes the journal.
                    if action == Hotkey::Journal
                        && exa_ui::game_gui::journal_leaves_combat(!word.dying(), !word.is_dead())
                        && pb.combat
                    {
                        pb.set_combat(false);
                        info!("journal: the opener leaves combat mode");
                    }
                    screens.write(ScreenRequest::screen(
                        if action == Hotkey::Inventory {
                            GameScreen::Inventory
                        } else {
                            GameScreen::Journal
                        },
                        !word.dying(),
                        !word.is_dead(),
                    ));
                }
                Hotkey::PowerBar => {
                    bars.write(BarCommand {
                        caster: entity,
                        action: BarAction::Toggle,
                    });
                }
                Hotkey::Power(index) => {
                    bars.write(BarCommand {
                        caster: entity,
                        action: if index < 0 {
                            BarAction::Use
                        } else {
                            BarAction::Pick(index as usize)
                        },
                    });
                }
                Hotkey::StepPower(direction) => {
                    bars.write(BarCommand {
                        caster: entity,
                        action: BarAction::Step(i32::from(direction)),
                    });
                }
                // The dialogue keys: the Talk key ends a running dialogue or
                // starts one with the focused character, the digits pick a
                // reply.
                Hotkey::EndDialogue | Hotkey::Choose(_) | Hotkey::Talk => {
                    match dialogue_command(action, dialogue.focus.as_deref().and_then(|f| f.0)) {
                        Some(DialogueCommand::Talk(who)) => {
                            dialogue.talk.write(exa_script::Talk(who));
                        }
                        Some(DialogueCommand::Pick(n)) => {
                            dialogue.pick.write(exa_script::PickReply(n));
                        }
                        Some(DialogueCommand::Leave) => {
                            dialogue.leave.write(exa_script::LeaveDialogue);
                        }
                        None => {}
                    }
                }
                // The skills key opens the skill screen
                // ([`crate::skill_screen`]); the power tree has its own key
                // reader.
                Hotkey::Skills => {
                    skills.write(crate::skill_screen::OpenSkills);
                }
                Hotkey::ConfirmEndMenu => {
                    confirms.write(crate::end_menu_screen::EndMenuConfirm);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAB: u32 = 0x09;
    const T: u32 = 0x54;

    fn keys() -> Bindings {
        Bindings::default()
    }

    /// In play: Tab, I, R, F, K and J do their thing; P and C need powers;
    /// T talks; an unbound key does nothing.
    #[test]
    fn the_play_keys() {
        let b = keys();
        let c = Context::default();
        let on = |vk: u32| hotkey(vk, &b, c);
        assert_eq!(on(TAB), Some(Hotkey::ToggleCombat));
        assert_eq!(on(0x49), Some(Hotkey::Inventory));
        assert_eq!(on(0x52), Some(Hotkey::SwapSets));
        assert_eq!(on(0x46), Some(Hotkey::ToggleLight));
        assert_eq!(on(0x4B), Some(Hotkey::Skills));
        assert_eq!(on(0x4A), Some(Hotkey::Journal));
        assert_eq!(on(T), Some(Hotkey::Talk));
        assert_eq!(on(0x51), Some(Hotkey::Power(-1)));
        assert_eq!(on(0x50), None);
        assert_eq!(on(0x43), None);
        assert_eq!(on(0x5A), None);
        let powers = Context { powers: 2, ..c };
        assert_eq!(hotkey(0x50, &b, powers), Some(Hotkey::PowerTree));
        assert_eq!(hotkey(0x43, &b, powers), Some(Hotkey::PowerBar));
        let companion = Context {
            companion_powers: Some(1),
            ..c
        };
        assert_eq!(hotkey(0x43, &b, companion), Some(Hotkey::PowerBar));
        assert_eq!(hotkey(0x50, &b, companion), None);
    }

    /// A dead player only ends a dialogue.
    #[test]
    fn the_dead_only_end_a_dialogue() {
        let b = keys();
        let dead = Context {
            dead: true,
            ..Context::default()
        };
        assert_eq!(hotkey(TAB, &b, dead), None);
        let talking = Context {
            dialogue: true,
            ..dead
        };
        assert_eq!(hotkey(TAB, &b, talking), Some(Hotkey::EndDialogue));
    }

    /// With a dialogue open the digits choose and T ends it; with the power
    /// bar open the digits pick ('0' the tenth), Q uses and the arrows step;
    /// other keys go on as in play.
    #[test]
    fn dialogue_and_power_bar_keys() {
        let b = keys();
        let talking = Context {
            dialogue: true,
            ..Context::default()
        };
        assert_eq!(hotkey(0x31, &b, talking), Some(Hotkey::Choose(0)));
        assert_eq!(hotkey(0x39, &b, talking), Some(Hotkey::Choose(8)));
        assert_eq!(hotkey(0x30, &b, talking), None);
        assert_eq!(hotkey(T, &b, talking), Some(Hotkey::EndDialogue));
        assert_eq!(hotkey(TAB, &b, talking), Some(Hotkey::ToggleCombat));
        let bar = Context {
            power_bar: true,
            ..Context::default()
        };
        assert_eq!(hotkey(0x30, &b, bar), Some(Hotkey::Power(9)));
        assert_eq!(hotkey(0x32, &b, bar), Some(Hotkey::Power(1)));
        assert_eq!(hotkey(0x51, &b, bar), Some(Hotkey::Power(-1)));
        assert_eq!(hotkey(0x27, &b, bar), Some(Hotkey::StepPower(1)));
        assert_eq!(hotkey(0x25, &b, bar), Some(Hotkey::StepPower(-1)));
        // T with the bar open is not Talk: it falls through to nothing.
        assert_eq!(hotkey(T, &b, bar), None);
    }

    /// The permission mask gates the windows and the swap; a mode hiding
    /// the inventory still opens it while collapsing.
    #[test]
    fn the_permissions_gate_the_windows() {
        let b = keys();
        let none = Context {
            allowed: 0,
            ..Context::default()
        };
        for vk in [0x49, 0x52, 0x4B, 0x4A, T, 0x51] {
            assert_eq!(hotkey(vk, &b, none), None, "{vk:#x}");
        }
        assert_eq!(hotkey(TAB, &b, none), Some(Hotkey::ToggleCombat));
        assert_eq!(hotkey(0x46, &b, none), Some(Hotkey::ToggleLight));
        let hidden = Context {
            hides_inventory: true,
            ..Context::default()
        };
        assert_eq!(hotkey(0x49, &b, hidden), None);
        let falling = Context {
            collapsing: true,
            ..hidden
        };
        assert_eq!(hotkey(0x49, &b, falling), Some(Hotkey::Inventory));
    }

    /// The pad's commands in the per-frame routine's order and gates: the
    /// bar only with the player's own powers, UsePower gated by the mask
    /// only with the bar closed, the steps only with it open and only when
    /// UsePower is not pressed, the tree with powers allowed and known, the
    /// swap and inventory under the key handler's gates, Skills ungated.
    #[test]
    fn the_pad_commands() {
        use crate::gamepad::command as c;
        let play = Context {
            powers: 2,
            ..Context::default()
        };
        assert_eq!(
            pad_hotkeys(c::POWER_BAR | c::USE_POWER | c::POWER_TREE, play),
            vec![Hotkey::PowerBar, Hotkey::Power(-1), Hotkey::PowerTree]
        );
        // The companion's powers do not open the bar from the pad.
        let companion = Context {
            powers: 0,
            companion_powers: Some(3),
            ..Context::default()
        };
        assert!(pad_hotkeys(c::POWER_BAR | c::POWER_TREE, companion).is_empty());
        // Select confirms the end menu after the bar, not in a dialogue.
        assert_eq!(
            pad_hotkeys(c::USE | c::POWER_BAR, play),
            vec![Hotkey::PowerBar, Hotkey::ConfirmEndMenu]
        );
        let talking = Context {
            dialogue: true,
            ..play
        };
        assert!(pad_hotkeys(c::USE, talking).is_empty());
        let bar = Context {
            power_bar: true,
            allowed: 0,
            ..play
        };
        assert_eq!(
            pad_hotkeys(c::USE_POWER | c::STEP_RIGHT, bar),
            vec![Hotkey::Power(-1)]
        );
        assert_eq!(
            pad_hotkeys(c::STEP_RIGHT | c::STEP_LEFT, bar),
            vec![Hotkey::StepPower(1)]
        );
        assert_eq!(pad_hotkeys(c::STEP_LEFT, bar), vec![Hotkey::StepPower(-1)]);
        // The steps mean nothing with the bar closed.
        assert!(pad_hotkeys(c::STEP_LEFT | c::STEP_RIGHT, play).is_empty());
        let none = Context { allowed: 0, ..play };
        assert_eq!(
            pad_hotkeys(
                c::USE_POWER | c::POWER_TREE | c::SWAP_WEAPON | c::INVENTORY | c::SKILLS,
                none
            ),
            vec![Hotkey::Skills]
        );
        assert_eq!(
            pad_hotkeys(c::SWAP_WEAPON | c::INVENTORY, play),
            vec![Hotkey::SwapSets, Hotkey::Inventory]
        );
        let hidden = Context {
            hides_inventory: true,
            ..play
        };
        assert!(pad_hotkeys(c::INVENTORY, hidden).is_empty());
        let falling = Context {
            collapsing: true,
            ..hidden
        };
        assert_eq!(pad_hotkeys(c::INVENTORY, falling), vec![Hotkey::Inventory]);
        // No death test; Talk, the dialogue's Select and the pause are
        // other systems'.
        let dead = Context { dead: true, ..play };
        assert_eq!(pad_hotkeys(c::INVENTORY, dead), vec![Hotkey::Inventory]);
        assert!(pad_hotkeys(c::TALK | c::PAUSE | c::LOOK, play).is_empty());
        // Combat toggles combat mode with no gate, after the power tree and
        // before the swap.
        assert_eq!(
            pad_hotkeys(c::COMBAT, Context { allowed: 0, ..dead }),
            vec![Hotkey::ToggleCombat]
        );
        let powered = Context { powers: 1, ..play };
        assert_eq!(
            pad_hotkeys(c::SWAP_WEAPON | c::COMBAT | c::POWER_TREE, powered),
            vec![Hotkey::PowerTree, Hotkey::ToggleCombat, Hotkey::SwapSets]
        );
    }

    /// The plugin's systems run three updates with and without a caster.
    #[test]
    fn the_hotkey_plugin_runs_with_and_without_a_caster() {
        for with_caster in [false, true] {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Bindings>()
                .add_plugins(HotkeyPlugin);
            let mut entity = app.world_mut().spawn(PlayerBody::new(0.0));
            if with_caster {
                entity.insert(PowerCaster::new(&[0x201]));
            }
            for _ in 0..3 {
                app.update();
            }
        }
    }

    /// C, a digit, Q and the arrows send the bar's commands for the caster.
    #[test]
    fn the_power_bar_keys_send_commands() {
        let run = |key: KeyCode, bar_open: bool| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Bindings>()
                .init_resource::<Hand>()
                .add_plugins(HotkeyPlugin);
            let caster = app
                .world_mut()
                .spawn((PlayerBody::new(0.0), PowerCaster::new(&[0x201])))
                .id();
            app.world_mut()
                .get_mut::<PowerCaster>(caster)
                .unwrap()
                .bar_open = bar_open;
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            let messages = app.world().resource::<Messages<BarCommand>>();
            let out: Vec<BarCommand> = messages.iter_current_update_messages().copied().collect();
            (caster, out)
        };
        let (caster, out) = run(KeyCode::KeyC, false);
        assert_eq!(
            out,
            vec![BarCommand {
                caster,
                action: BarAction::Toggle
            }]
        );
        assert_eq!(run(KeyCode::Digit2, true).1[0].action, BarAction::Pick(1));
        assert_eq!(run(KeyCode::KeyQ, true).1[0].action, BarAction::Use);
        assert_eq!(run(KeyCode::KeyQ, false).1[0].action, BarAction::Use);
        assert_eq!(
            run(KeyCode::ArrowLeft, true).1[0].action,
            BarAction::Step(-1)
        );
        assert_eq!(
            run(KeyCode::ArrowRight, true).1[0].action,
            BarAction::Step(1)
        );
    }

    /// In the running game Tab toggles the player's combat mode, but not a
    /// dead player's.
    #[test]
    fn tab_toggles_combat_unless_dead() {
        let run = |word: Condition| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Bindings>()
                .init_resource::<Hand>()
                .add_message::<ScreenRequest>()
                .add_message::<crate::end_menu_screen::EndMenuConfirm>()
                .add_message::<crate::skill_screen::OpenSkills>()
                .add_message::<BarCommand>()
                .add_message::<FlipWeaponRequest>()
                .init_resource::<DialogueFocus>()
                .add_message::<exa_script::Talk>()
                .add_message::<exa_script::PickReply>()
                .add_message::<exa_script::LeaveDialogue>()
                .init_resource::<exa_physics::sim_app::FixedCommands>()
                .add_systems(Update, press_hotkeys);
            let player = app.world_mut().spawn((PlayerBody::new(0.0), word)).id();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Tab);
            app.update();
            app.world().get::<PlayerBody>(player).unwrap().combat
        };
        assert!(run(Condition::NORMAL));
        assert!(!run(Condition::DEAD));
    }

    /// The swap key asks for the weapon switch (the requested motion)
    /// unless an item drag of the GUI is in flight (the swap gate's drag
    /// test).
    #[test]
    fn a_gui_drag_refuses_the_swap_key() {
        let run = |dragging: bool| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Bindings>()
                .init_resource::<Hand>()
                .add_message::<ScreenRequest>()
                .add_message::<crate::end_menu_screen::EndMenuConfirm>()
                .add_message::<crate::skill_screen::OpenSkills>()
                .add_message::<BarCommand>()
                .add_message::<FlipWeaponRequest>()
                .init_resource::<DialogueFocus>()
                .add_message::<exa_script::Talk>()
                .add_message::<exa_script::PickReply>()
                .add_message::<exa_script::LeaveDialogue>()
                .init_resource::<exa_physics::sim_app::FixedCommands>()
                .init_resource::<crate::panel_live::PanelDrag>()
                .add_systems(Update, press_hotkeys);
            if dragging {
                app.world_mut()
                    .resource_mut::<crate::panel_live::PanelDrag>()
                    .source = Some(crate::panel_live::Panel::PlayerPack);
            }
            let player = app
                .world_mut()
                .spawn((
                    PlayerBody::new(0.0),
                    Condition::NORMAL,
                    Possessions::default(),
                    exa_anim::Animator::new("Default", "Idle"),
                ))
                .id();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyR);
            app.update();
            app.world()
                .get::<exa_anim::Animator>(player)
                .unwrap()
                .requested
                == 0x1F
        };
        assert!(run(false));
        assert!(!run(true));
    }

    /// The spirit form's switch leaves the hotkeys only the powers': the
    /// inventory key asks for nothing while it holds the controls (and the
    /// dialogue is ended), and asks again once the way home lets them go.
    #[test]
    fn the_spirit_form_keeps_only_the_powers_keys() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Bindings>()
            .init_resource::<Hand>()
            .init_resource::<PlayerControl>()
            .init_resource::<crate::gui_screens::GameScreens>()
            .add_message::<ScreenRequest>()
            .add_message::<crate::end_menu_screen::EndMenuConfirm>()
            .add_message::<crate::skill_screen::OpenSkills>()
            .add_message::<BarCommand>()
            .add_message::<FlipWeaponRequest>()
            .init_resource::<DialogueFocus>()
            .add_message::<exa_script::Talk>()
            .add_message::<exa_script::PickReply>()
            .add_message::<exa_script::LeaveDialogue>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, press_hotkeys);
        app.world_mut()
            .resource_mut::<PlayerControl>()
            .set_off(false);
        app.world_mut()
            .spawn((PlayerBody::new(0.0), Condition::NORMAL));
        let inventory_requests = |app: &mut App| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyI);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::KeyI);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
            let mut messages = app.world_mut().resource_mut::<Messages<ScreenRequest>>();
            messages
                .drain()
                .filter(|r| r.screen == GameScreen::Inventory)
                .count()
        };
        crate::controls_live::spirit_controls(app.world_mut(), true);
        assert_eq!(
            app.world()
                .resource::<Messages<exa_script::LeaveDialogue>>()
                .len(),
            1,
            "the switch ends the dialogue"
        );
        assert!(app.world().resource::<Hand>().keeps_left);
        assert_eq!(inventory_requests(&mut app), 0);
        crate::controls_live::spirit_controls(app.world_mut(), false);
        assert_eq!(inventory_requests(&mut app), 1);
    }

    /// I and J ask for the inventory and the journal; a dead player asks
    /// for nothing.
    #[test]
    fn the_screen_keys_send_requests() {
        let run = |word: Condition, key: KeyCode| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Bindings>()
                .init_resource::<Hand>()
                .add_message::<ScreenRequest>()
                .add_message::<crate::end_menu_screen::EndMenuConfirm>()
                .add_message::<crate::skill_screen::OpenSkills>()
                .add_message::<BarCommand>()
                .add_message::<FlipWeaponRequest>()
                .init_resource::<DialogueFocus>()
                .add_message::<exa_script::Talk>()
                .add_message::<exa_script::PickReply>()
                .add_message::<exa_script::LeaveDialogue>()
                .init_resource::<exa_physics::sim_app::FixedCommands>()
                .add_systems(Update, press_hotkeys);
            app.world_mut().spawn((PlayerBody::new(0.0), word));
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            let messages = app.world().resource::<Messages<ScreenRequest>>();
            messages
                .iter_current_update_messages()
                .copied()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            run(Condition::NORMAL, KeyCode::KeyI),
            vec![ScreenRequest::screen(GameScreen::Inventory, true, true)]
        );
        assert_eq!(
            run(Condition::NORMAL, KeyCode::KeyJ)[0].screen,
            GameScreen::Journal
        );
        assert!(run(Condition::DEAD, KeyCode::KeyI).is_empty());
    }

    /// The swap key queues the weapon switch in the body's requested motion
    /// (the clip's swap point trades the sets later); with the player's own
    /// inventory open the sets swap at once; a busy body asks for nothing.
    #[test]
    fn the_swap_key_queues_the_switch_unless_the_pack_is_open() {
        let run = |word: Condition, pack_open: bool| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Bindings>()
                .init_resource::<Hand>()
                .init_resource::<crate::gui_screens::GameScreens>()
                .add_message::<ScreenRequest>()
                .add_message::<crate::end_menu_screen::EndMenuConfirm>()
                .add_message::<crate::skill_screen::OpenSkills>()
                .add_message::<BarCommand>()
                .add_message::<FlipWeaponRequest>()
                .init_resource::<DialogueFocus>()
                .add_message::<exa_script::Talk>()
                .add_message::<exa_script::PickReply>()
                .add_message::<exa_script::LeaveDialogue>()
                .init_resource::<exa_physics::sim_app::FixedCommands>()
                .add_systems(Update, press_hotkeys);
            if pack_open {
                let mut screens = app
                    .world_mut()
                    .resource_mut::<crate::gui_screens::GameScreens>();
                let player = screens.player;
                screens.show_inventory(true, true, 8, false, player);
                assert!(screens.is_open(GameScreen::Inventory));
            }
            let player = app
                .world_mut()
                .spawn((
                    PlayerBody::new(0.0),
                    word,
                    Possessions::default(),
                    exa_anim::Animator::new("Default", "Idle"),
                ))
                .id();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyR);
            app.update();
            (
                app.world()
                    .get::<exa_anim::Animator>(player)
                    .unwrap()
                    .requested,
                app.world()
                    .get::<exa_items::catalogue::SwapRequested>(player)
                    .is_some(),
            )
        };
        assert_eq!(run(Condition::NORMAL, false), (0x1F, false));
        assert_eq!(run(Condition::NORMAL, true), (0, true));
        assert_eq!(run(Condition::SWITCHING, false), (0, false));
        assert_eq!(run(Condition::DOWN, false), (0, false));
    }

    /// Entering combat clears the backpedal, keys' heading and turn-about
    /// bits and lets go of what the hand holds.
    #[test]
    fn entering_combat_lets_go() {
        let mut hand = Hand {
            holding: Holding::Drag {
                body: Entity::PLACEHOLDER,
                height: 0.3,
                character: false,
            },
            ..Hand::default()
        };
        let mut req = Locomotion {
            flags: flags::MOVE | flags::BACK | flags::KEYS | flags::TURN_ABOUT | flags::RUN,
            ..Locomotion::default()
        };
        assert_eq!(enter_combat(&mut hand, &mut req), Some(Entity::PLACEHOLDER));
        assert_eq!(req.flags, flags::MOVE | flags::RUN);
        assert_eq!(hand.holding, Holding::Nothing);
        hand.holding = Holding::Lift;
        assert_eq!(enter_combat(&mut hand, &mut req), None);
        assert_eq!(hand.holding, Holding::Nothing);
    }

    fn ball_body(at: Vec3) -> Ragdoll {
        Ragdoll::rigid(&[exa_physics::body::RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::from_translation(at),
            rest: [
                Vec3::new(-0.2, 0.0, -0.2),
                Vec3::new(0.2, 0.0, -0.2),
                Vec3::new(0.0, 0.0, 0.2),
                Vec3::new(0.0, 0.4, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: bevy::math::Affine3A::IDENTITY,
            mobility: 1.0,
            spheres: vec![(Vec3::ZERO, 0.2)],
            origin: Vec3::ZERO,
            links: Vec::new(),
            mesh: None,
            triangles: None,
        }])
    }

    /// Tab into combat mode while an item is lifted: the live key handler
    /// empties the hand and the hold frame puts the item back at the frame
    /// its lift saved, as the original's lift cancel restores the saved
    /// frames (not where the cursor last placed it).
    #[test]
    fn entering_combat_puts_a_lifted_item_back() {
        use crate::control_nodes::Frame;
        use crate::item_cursor::{HeldItem, ItemCursor};
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Bindings>()
            .init_resource::<Hand>()
            .init_resource::<ItemCursor>()
            .init_resource::<Time<Real>>()
            .init_resource::<exa_ui::ScriptedPointer>()
            .init_resource::<exa_physics::prop::StandIns>()
            .init_resource::<exa_physics::ColliderNodes>()
            .init_resource::<exa_physics::collision_share::FramePhysics>()
            .add_message::<ScreenRequest>()
            .add_message::<crate::end_menu_screen::EndMenuConfirm>()
            .add_message::<crate::skill_screen::OpenSkills>()
            .add_message::<BarCommand>()
            .add_message::<FlipWeaponRequest>()
            .init_resource::<DialogueFocus>()
            .add_message::<exa_script::Talk>()
            .add_message::<exa_script::PickReply>()
            .add_message::<exa_script::LeaveDialogue>()
            .add_message::<crate::hand::WorldClick>()
            .add_message::<crate::object_click::ObjectClick>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, (press_hotkeys, crate::hand::hold_hand).chain());
        app.world_mut()
            .spawn((PlayerBody::new(0.0), Condition::NORMAL));
        let saved_at = Vec3::new(1.0, 0.5, 2.0);
        let saved = bevy::math::Affine3A::from_translation(saved_at);
        // The item hangs on the cursor, away from where the lift took it.
        let item = app
            .world_mut()
            .spawn(ball_body(Vec3::new(3.0, 1.5, 0.0)))
            .id();
        app.world_mut()
            .resource_mut::<ItemCursor>()
            .begin(HeldItem {
                body: item,
                part: 0,
                saved: vec![saved],
                saved_frame: Frame::new(saved_at, Mat3::IDENTITY),
                shown: Frame::new(Vec3::new(3.0, 1.5, 0.0), Mat3::IDENTITY),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.2),
                target: Vec3::new(3.0, 1.5, 0.0),
                last: Some(Frame::new(Vec3::new(3.0, 1.5, 0.0), Mat3::IDENTITY)),
                item: 0,
                model: 0,
                from_lock: None,
            });
        app.world_mut().resource_mut::<Hand>().holding = Holding::Lift;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert_eq!(app.world().resource::<Hand>().holding, Holding::Nothing);
        assert!(app.world().resource::<ItemCursor>().held.is_none());
        assert!(!app.world().resource::<Hand>().cancel_lift);
        let at = Vec3::from(
            app.world().get::<Ragdoll>(item).unwrap().parts[0]
                .transform()
                .translation,
        );
        assert!((at - saved_at).length() < 1e-4, "the item is at {at}");
    }

    /// The dialogue keys: with the panel open the Talk key ends, the digits
    /// pick (from zero); with nobody focused the Talk key sends nothing.
    #[test]
    fn the_dialogue_keys_send_their_commands() {
        assert_eq!(
            dialogue_command(Hotkey::EndDialogue, None),
            Some(DialogueCommand::Leave)
        );
        assert_eq!(
            dialogue_command(Hotkey::Choose(0), None),
            Some(DialogueCommand::Pick(0))
        );
        assert_eq!(
            dialogue_command(Hotkey::Choose(8), None),
            Some(DialogueCommand::Pick(8))
        );
        assert_eq!(
            dialogue_command(Hotkey::Talk, Some(9)),
            Some(DialogueCommand::Talk(9))
        );
        assert_eq!(dialogue_command(Hotkey::Talk, None), None);
        assert_eq!(dialogue_command(Hotkey::Inventory, Some(9)), None);
    }
}
