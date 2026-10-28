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
//! power bar's commands to [`crate::thaum::cast`]. The skills screen's
//! opening is not in Exarust, the power tree has its own key reader, and
//! there is no dialogue.
//!
//! Evidence: the controls' key handler, the combat toggle and the swap and
//! light requests, read in the instructions
//! (`context/re/player-control-hand.md`, Hotkeys).

use bevy::prelude::*;
use exa_character::grip_runtime::FlipWeaponRequest;
use exa_character::held::Hands;
use exa_combat::Condition;
use exa_items::possession::{LIT, Possessions, SwapRequest};
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::{Locomotion, flags};

use exa_ui::game_gui::GameScreen;

use crate::bindings::{Action, Bindings, virtual_key};
use crate::gui_screens::ScreenRequest;
use crate::hand::{Hand, Holding};
use crate::player::PlayerBody;
use crate::thaum::cast::{BarAction, BarCommand, PowerCaster};

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

/// Entering combat mode: the hand lets go and the request's backpedal,
/// keys' heading and turn-about bits clear. The original's cancel puts a
/// lifted item back where its lift saved it and leaves a drag's link in
/// place; Exarust lets go of the drag.
pub fn enter_combat(hand: &mut Hand, req: &mut Locomotion) -> Option<Entity> {
    req.flags &= !COMBAT_CLEARS;
    let held = match hand.holding {
        Holding::Drag { body, .. } => Some(body),
        _ => None,
    };
    hand.holding = Holding::Nothing;
    held
}

pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ScreenRequest>()
            .add_message::<BarCommand>()
            .add_message::<FlipWeaponRequest>()
            .add_systems(
                Update,
                press_hotkeys
                    .in_set(HotkeySet)
                    .after(crate::player::cursor_ground)
                    .before(crate::player::player_combat),
            );
    }
}

/// The system that reads the player's hotkeys.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HotkeySet;

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
    )>,
    casters: Query<&PowerCaster>,
    mut bodies: Query<&mut Ragdoll, Without<PlayerBody>>,
    mut screens: MessageWriter<ScreenRequest>,
    mut bars: MessageWriter<BarCommand>,
    mut flips: MessageWriter<FlipWeaponRequest>,
) {
    let mut pressed: Vec<u32> = keys
        .get_just_pressed()
        .filter_map(|&k| virtual_key(k))
        .collect();
    // The pad's presses go through the same handler as their keys. The
    // original has no Steam action for combat mode; Exarust's pad turns it
    // on and off with Look (the right stick's click), which has no Cursor
    // Look meaning on a gamepad (Exarust's own). SwapWeapon is the
    // original's action for the weapon-set swap. Each counts as its bound
    // key, so the handler's gates apply to it.
    if pad_status.as_ref().is_some_and(|s| s.active())
        && let Some(frame) = pad.as_ref().map(|p| p.frame())
    {
        for (on, action) in [
            (frame.look_pressed, Action::ToggleCombat),
            (
                frame.pressed & crate::gamepad::command::SWAP_WEAPON != 0,
                Action::SwitchEquipped,
            ),
        ] {
            let key = bindings.key(action);
            if on && key != 0 {
                pressed.push(key);
            }
        }
    }
    if pressed.is_empty() {
        return;
    }
    let mut hand = hand;
    for (entity, mut pb, mut req, condition, hands, mut possessions) in &mut players {
        let word = condition.copied().unwrap_or_default();
        let caster = casters.get(entity).ok();
        let context = Context {
            dead: word.is_dead(),
            collapsing: word.dying(),
            power_bar: caster.is_some_and(|c| c.bar_open),
            powers: caster.map_or(0, |c| c.known.len() as i32),
            ..Context::default()
        };
        for &key in &pressed {
            let Some(action) = hotkey(key, &bindings, context) else {
                continue;
            };
            debug!("hotkey {key:#x}: {action:?}");
            match action {
                Hotkey::ToggleCombat => {
                    pb.toggle_combat();
                    pb.aim = req.heading;
                    if pb.combat
                        && let Some(hand) = hand.as_deref_mut()
                        && let Some(body) = enter_combat(hand, &mut req)
                        && let Ok(mut ragdoll) = bodies.get_mut(body)
                    {
                        ragdoll.release_grab();
                    }
                }
                Hotkey::SwapSets => {
                    let lifting = hand.as_deref().is_some_and(|h| h.holding == Holding::Lift);
                    if Possessions::swap_request(word.0, lifting, false) != SwapRequest::Refused {
                        commands
                            .entity(entity)
                            .insert(exa_items::catalogue::SwapRequested);
                    }
                }
                Hotkey::ToggleLight => {
                    let first =
                        hands.and_then(|h| h.main.map(|m| m.light).or(h.off_light.then_some(true)));
                    if let Some(p) = possessions.as_deref_mut() {
                        let burning = p.out[0].flags & LIT != 0;
                        // The original's branch ends by starting the held
                        // item's flip.
                        if p.toggle_light(
                            first.is_some(),
                            word.0 & Condition::SWITCHING.0 != 0,
                            first.unwrap_or(false),
                            burning,
                        ) {
                            flips.write(FlipWeaponRequest { body: entity });
                        }
                    }
                }
                Hotkey::Inventory | Hotkey::Journal => {
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
                // No skills screen or dialogue to open yet (the power tree
                // has its own key reader).
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
                .add_message::<BarCommand>()
                .add_message::<FlipWeaponRequest>()
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
                .add_message::<BarCommand>()
                .add_message::<FlipWeaponRequest>()
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
}
