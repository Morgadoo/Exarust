//! Tests for the gamepad action sets, the menu reader, the gameplay reader
//! and the default bindings.

use super::*;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

fn pad() -> PadSample {
    PadSample {
        connected: true,
        ..PadSample::default()
    }
}

fn menu_ctx() -> MenuContext {
    MenuContext {
        focused: true,
        ready: true,
        ..MenuContext::default()
    }
}

fn play_ctx() -> PlayContext {
    PlayContext::default()
}

fn primed_play() -> PlayPad {
    let mut play = PlayPad::default();
    play.update(0, &PadSample::default(), play_ctx());
    play.update(3, &PadSample::default(), play_ctx());
    play
}

#[test]
fn the_action_sets_hold_the_originals_actions() {
    assert_eq!(Id::ALL.len(), 28);
    for (index, id) in Id::ALL.iter().enumerate() {
        assert_eq!(id.index(), index);
    }
    assert_eq!(Id::MenuNav.kind(), Kind::Analog);
    assert_eq!(Id::Move.kind(), Kind::Analog);
    assert_eq!(Id::Combat.kind(), Kind::Digital);
    assert!(Id::Move.in_set(Set::Interact) && Id::Move.in_set(Set::Combat));
    assert!(!Id::Move.in_set(Set::Menus));
    assert!(Id::MenuSelect.in_set(Set::Menus));
    assert!(!Id::MenuSelect.in_set(Set::Interact));
    assert!(Id::Inventory.in_set(Set::Interact));
    assert!(!Id::Inventory.in_set(Set::Combat));
    assert!(Id::Crouch.in_set(Set::Combat));
    assert!(!Id::Crouch.in_set(Set::Interact));
    assert_eq!(Id::Move.manifest_name(Set::Interact), "MoveI");
    assert_eq!(Id::Move.manifest_name(Set::Combat), "MoveC");
    assert_eq!(Id::Aim.manifest_name(Set::Combat), "AimC");
    assert_eq!(Id::Alternate.manifest_name(Set::Combat), "AltAction");
    assert_eq!(Id::Crouch.manifest_name(Set::Combat), "CrouchC");
    assert_eq!(Id::Inventory.manifest_name(Set::Interact), "Inventory");
    assert_eq!(Id::MenuExit.manifest_name(Set::Menus), "MenuExit");
    assert_eq!(Id::MenuLeftButton.manifest_name(Set::Menus), "MenuLMB");
    // The default table has one entry per action and binds the mode button.
    assert_eq!(DEFAULT_CONTROLS.len(), Id::COUNT);
    assert_eq!(
        default_control(Id::CameraMode),
        Control::Button(GamepadButton::Mode)
    );
    assert_eq!(
        default_control(Id::Move),
        Control::Stick(GamepadAxis::LeftStickX, GamepadAxis::LeftStickY)
    );
}

#[test]
fn the_menu_stick_dead_zone_is_a_squared_magnitude_of_a_quarter() {
    let mut menu = MenuPad::default();
    let edge = pad().with_analog(Id::MenuNav, Vec2::new(0.0, 0.5));
    assert_eq!(menu.frame(16, &edge, menu_ctx()).key, 0);
    let over = pad().with_analog(Id::MenuNav, Vec2::new(0.0, 0.5001));
    assert_eq!(menu.frame(32, &over, menu_ctx()).key, gui_key::UP | 0x10);
}

#[test]
fn the_menu_sticks_directions_take_the_dominant_axis() {
    let dir = |x: f32, y: f32| {
        let mut menu = MenuPad::default();
        let sample = pad().with_analog(Id::MenuNav, Vec2::new(x, y));
        menu.frame(16, &sample, menu_ctx()).key & 0xf000
    };
    assert_eq!(dir(0.6, 0.0), 0x2000);
    assert_eq!(dir(-0.6, 0.0), 0x1000);
    assert_eq!(dir(0.0, 0.6), 0x4000);
    assert_eq!(dir(0.0, -0.6), 0x8000);
    // A tie goes to the vertical axis.
    assert_eq!(dir(0.6, 0.6), 0x4000);
    assert_eq!(dir(-0.6, -0.6), 0x8000);
}

#[test]
fn the_menu_steps_repeat_after_400_then_every_150_ms() {
    let sample = pad().with_analog(Id::MenuNav, Vec2::new(0.0, 0.6));
    let mut menu = MenuPad::default();
    assert_eq!(menu.frame(16, &sample, menu_ctx()).key, gui_key::UP | 0x10);
    assert_eq!(menu.frame(100, &sample, menu_ctx()).key, 0);
    assert_eq!(menu.frame(416, &sample, menu_ctx()).key, 0);
    assert_eq!(menu.frame(417, &sample, menu_ctx()).key, gui_key::UP | 0x10);
    assert_eq!(menu.frame(567, &sample, menu_ctx()).key, 0);
    assert_eq!(menu.frame(568, &sample, menu_ctx()).key, gui_key::UP | 0x10);
    // A stick that returns inside the dead zone resets the first delay.
    let rest = pad();
    menu.frame(600, &rest, menu_ctx());
    assert_eq!(menu.frame(700, &sample, menu_ctx()).key, gui_key::UP | 0x10);
    assert_eq!(menu.frame(1100, &sample, menu_ctx()).key, 0);
    assert_eq!(
        menu.frame(1101, &sample, menu_ctx()).key,
        gui_key::UP | 0x10
    );
    assert_eq!(menu.frame(1251, &sample, menu_ctx()).key, 0);
    assert_eq!(
        menu.frame(1252, &sample, menu_ctx()).key,
        gui_key::UP | 0x10
    );
}

#[test]
fn a_fast_menu_navigation_form_steps_again_every_frame_after_its_second_step() {
    let sample = pad().with_analog(Id::MenuNav, Vec2::new(0.6, 0.0));
    let ctx = MenuContext {
        fast_nav: true,
        ..menu_ctx()
    };
    let mut menu = MenuPad::default();
    assert_eq!(menu.frame(16, &sample, ctx).key, gui_key::RIGHT | 0x10);
    assert_eq!(menu.frame(100, &sample, ctx).key, 0);
    assert_eq!(menu.frame(417, &sample, ctx).key, gui_key::RIGHT | 0x10);
    // The original writes the fast delay to the scroll stick's deadline and
    // leaves the navigation deadline alone, so navigation runs on freely.
    assert_eq!(menu.frame(418, &sample, ctx).key, gui_key::RIGHT | 0x10);
}

#[test]
fn the_scroll_stick_uses_its_own_codes_and_is_skipped_while_navigation_steps() {
    let mut menu = MenuPad::default();
    let scroll = pad().with_analog(Id::MenuScreen, Vec2::new(0.6, 0.0));
    assert_eq!(
        menu.frame(16, &scroll, menu_ctx()).key,
        gui_key::SCROLL_RIGHT | 0x20
    );
    // The navigation stick's step takes the frame: the scroll stick is not
    // read at all then.
    let both = scroll.with_analog(Id::MenuNav, Vec2::new(0.0, 0.6));
    let mut menu = MenuPad::default();
    assert_eq!(menu.frame(16, &both, menu_ctx()).key, gui_key::UP | 0x10);
}

#[test]
fn the_menu_cursor_is_the_stick_scaled_by_sixteenths_of_a_millisecond() {
    let sample = pad().with_analog(Id::MenuNav, Vec2::new(0.5, 0.25));
    let mut menu = MenuPad::default();
    let frame = menu.frame(32, &sample, menu_ctx());
    // 32 ms at 0.0625 is a factor of 2; the vertical axis is flipped to
    // down-positive.
    assert_eq!(frame.cursor, Vec2::new(1.0, -0.5));
}

#[test]
fn menu_keys_only_dispatch_on_their_first_frame_and_a_new_set_primes_them() {
    let select = pad().with_digital(Id::MenuSelect, true);
    let mut menu = MenuPad::default();
    assert_eq!(menu.frame(16, &select, menu_ctx()).key, gui_key::SELECT);
    assert_eq!(menu.frame(17, &select, menu_ctx()).key, 0);
    assert_eq!(menu.frame(18, &pad(), menu_ctx()).key, 0);
    assert_eq!(menu.frame(19, &select, menu_ctx()).key, gui_key::SELECT);
    assert_eq!(
        menu.frame(20, &select.with_digital(Id::MenuBack, true), menu_ctx())
            .key,
        gui_key::FORWARD
    );
    let mut menu = MenuPad::default();
    menu.activate(100);
    assert_eq!(menu.frame(100, &select, menu_ctx()).key, 0);
    assert_eq!(menu.frame(102, &select, menu_ctx()).key, 0);
    assert_eq!(menu.frame(103, &select, menu_ctx()).key, 0);
    assert_eq!(menu.frame(104, &pad(), menu_ctx()).key, 0);
    assert_eq!(menu.frame(105, &select, menu_ctx()).key, gui_key::SELECT);
}

#[test]
fn a_directional_pick_takes_the_nearest_control_of_the_dominant_axis() {
    let targets = [
        PickTarget {
            id: 1,
            centre: IVec2::new(0, 100),
            reachable: true,
            current: false,
        },
        PickTarget {
            id: 2,
            centre: IVec2::new(40, 60),
            reachable: true,
            current: false,
        },
        PickTarget {
            id: 3,
            centre: IVec2::new(600, 40),
            reachable: true,
            current: false,
        },
        PickTarget {
            id: 4,
            centre: IVec2::new(0, 20),
            reachable: true,
            current: true,
        },
    ];
    assert_eq!(
        pick_control(Vec2::new(0.0, 1.0), IVec2::ZERO, &targets),
        Some(2)
    );
    // A target further sideways than 320 pixels is not straight below.
    assert_eq!(
        pick_control(Vec2::new(0.0, -1.0), IVec2::ZERO, &targets),
        None
    );
}

#[test]
fn a_horizontal_pick_scores_the_offset_with_its_vertical_part_stretched() {
    let targets = [
        PickTarget {
            id: 1,
            centre: IVec2::new(100, 0),
            reachable: true,
            current: false,
        },
        PickTarget {
            id: 2,
            centre: IVec2::new(300, 10),
            reachable: true,
            current: false,
        },
        PickTarget {
            id: 3,
            centre: IVec2::new(100, 100),
            reachable: true,
            current: false,
        },
    ];
    assert_eq!(
        pick_control(Vec2::new(1.0, 0.0), IVec2::ZERO, &targets),
        Some(1)
    );
    // With only the far-down target left, its stretched offset is under the
    // 0.3 alignment.
    assert_eq!(
        pick_control(Vec2::new(1.0, 0.0), IVec2::ZERO, &targets[2..]),
        None
    );
}

#[test]
fn the_focus_stack_moves_duplicates_and_picks_the_sets() {
    let mut stack = FocusStack::default();
    assert!(!stack.focused());
    assert_eq!(
        stack.push(FocusEntry {
            form: 7,
            flags: 0,
            full_screen: false
        }),
        FocusChange::Menus
    );
    assert_eq!(
        stack.push(FocusEntry {
            form: 8,
            flags: 0x8001,
            full_screen: false
        }),
        FocusChange::Gameplay(0)
    );
    assert_eq!(
        stack.push(FocusEntry {
            form: 9,
            flags: 0x8002,
            full_screen: true
        }),
        FocusChange::Gameplay(1)
    );
    assert_eq!(stack.depth(), 3);
    // A form pushed again moves to the top.
    assert_eq!(
        stack.push(FocusEntry {
            form: 7,
            flags: 0,
            full_screen: false
        }),
        FocusChange::Menus
    );
    assert_eq!(stack.depth(), 3);
    assert_eq!(stack.entries().last().map(|e| e.form), Some(7));
    assert_eq!(stack.pop(7), FocusChange::Gameplay(1));
    assert_eq!(stack.pop(9), FocusChange::Gameplay(0));
    assert_eq!(stack.pop(8), FocusChange::Menus);
    assert!(!stack.focused());
    assert_eq!(stack.pop(8), FocusChange::None);
    for form in 0..9 {
        stack.push(FocusEntry {
            form,
            flags: 0,
            full_screen: false,
        });
    }
    assert_eq!(stack.depth(), FocusStack::LIMIT);
}

#[test]
fn the_camera_stick_has_a_half_dead_zone_vertically_and_a_squared_tenth_horizontally() {
    assert_eq!(look_input(Vec2::new(0.0, 0.4999)), Vec2::ZERO);
    assert_eq!(look_input(Vec2::new(0.0, 0.5)), Vec2::new(0.0, 5.0));
    assert_eq!(look_input(Vec2::new(0.0, -0.5)), Vec2::new(0.0, -5.0));
    assert_eq!(look_input(Vec2::new(0.09, 0.0)), Vec2::ZERO);
    // 0.1 squared is exactly the 0.01 the original needs to be over.
    assert_eq!(look_input(Vec2::new(0.1, 0.0)), Vec2::ZERO);
    // Squared, the horizontal axis must then clear the 0.01 pair threshold,
    // so 0.2 is still inside the response's floor and 0.4 is out.
    assert_eq!(look_input(Vec2::new(0.2, 0.0)), Vec2::ZERO);
    assert_eq!(look_input(Vec2::new(0.4, 0.0)), Vec2::new(4.8, 0.0));
    assert_eq!(look_input(Vec2::new(-0.5, 0.0)), Vec2::new(-7.5, 0.0));
    // The vertical dead zone alone is enough for a look.
    assert_eq!(look_input(Vec2::new(0.0, 0.6)), Vec2::new(0.0, 6.0));
}

#[test]
fn the_look_rate_is_the_aim_speed_times_the_original_scale() {
    let look = look_input(Vec2::new(1.0, 1.0));
    assert_eq!(look, Vec2::new(30.0, 10.0));
    let delta = look_delta(look, 0.29);
    assert!(near(delta.x, -0.021_75), "{delta:?}");
    assert!(near(delta.y, -0.007_25), "{delta:?}");
    let full = look_delta(look, 1.0);
    assert!(near(full.x, -0.075));
    assert!(near(full.y, -0.025));
}

#[test]
fn the_zoom_action_moves_twenty_on_the_eight_hundred_to_two_thousand_scale() {
    assert!((zoom_step(10.0) - 20.0).abs() < 1e-4, "{}", zoom_step(10.0));
    assert!((zoom_step(-10.0) + 20.0).abs() < 1e-4);
}

#[test]
fn the_move_stick_turns_into_the_camera_frame_and_runs_past_its_threshold() {
    let sample = pad().with_analog(Id::Move, Vec2::new(0.5, 0.5));
    let mut play = primed_play();
    let frame = *play.update(4, &sample, play_ctx());
    assert_eq!(frame.move_raw, Vec2::new(0.5, 0.5));
    assert_eq!(frame.move_world, Vec3::new(0.5, 0.0, 0.5));
    assert!(!frame.move_fast);
    // Six tenths a side is 0.72, under the 0.8 run threshold.
    let slow = pad().with_analog(Id::Move, Vec2::new(0.6, 0.6));
    let frame = *play.update(5, &slow, play_ctx());
    assert!(!frame.move_fast);
    let fast = pad().with_analog(Id::Move, Vec2::new(0.81, 0.4));
    let frame = *play.update(6, &fast, play_ctx());
    assert!(frame.move_fast);
}

#[test]
fn the_aim_stick_turns_into_the_camera_frame_or_stays_raw() {
    let sample = pad().with_analog(Id::Aim, Vec2::new(0.0, 1.0));
    let mut play = primed_play();
    let frame = *play.update(4, &sample, play_ctx());
    assert_eq!(frame.aim_world, Vec3::Z);
    let ctx = PlayContext {
        aim_in_world: true,
        ..play_ctx()
    };
    let frame = *play.update(5, &sample, ctx);
    assert_eq!(frame.aim_world, Vec3::Z);
    let sample = pad().with_analog(Id::Aim, Vec2::new(1.0, 0.0));
    let frame = *play.update(6, &sample, ctx);
    assert_eq!(frame.aim_world, Vec3::X);
}

#[test]
fn the_attack_is_a_latched_edge_and_the_command_word_keeps_it() {
    let combat = pad().with_digital(Id::Combat, true);
    let mut play = primed_play();
    let frame = *play.update(4, &combat, play_ctx());
    assert_eq!(frame.pressed & command::ATTACK, command::ATTACK);
    assert!(play.attack_held());
    assert!(play.take_attack_press());
    assert!(!play.take_attack_press());
    let frame = *play.update(5, &combat, play_ctx());
    assert_eq!(frame.pressed & command::ATTACK, 0);
    assert_eq!(play.commands() & command::ATTACK, command::ATTACK);
    play.consume(command::ATTACK);
    assert_eq!(play.commands() & command::ATTACK, 0);
    assert!(play.attack_held());
}

#[test]
fn the_attack_is_not_latched_while_the_set_primes() {
    let combat = pad().with_digital(Id::Combat, true);
    let mut play = PlayPad::default();
    play.update(0, &combat, play_ctx());
    play.update(3, &combat, play_ctx());
    assert!(!play.take_attack_press());
    assert_eq!(play.commands() & command::ATTACK, 0);
    // Released and pressed again, it fires.
    play.update(4, &pad(), play_ctx());
    play.update(5, &combat, play_ctx());
    assert!(play.take_attack_press());
}

#[test]
fn a_step_mode_steps_every_three_hundred_ms() {
    let sample = pad().with_analog(Id::Move, Vec2::new(0.5, 0.0));
    let ctx = PlayContext {
        modes: StepModes {
            across: true,
            along: false,
        },
        ..play_ctx()
    };
    let mut play = primed_play();
    let frame = *play.update(4, &sample, ctx);
    assert_eq!(frame.step, command::STEP_RIGHT);
    assert_eq!(frame.move_world, Vec3::ZERO);
    play.consume(command::STEP_RIGHT);
    assert_eq!(play.update(5, &sample, ctx).pressed, 0);
    assert_eq!(play.update(304, &sample, ctx).pressed, 0);
    assert_eq!(play.update(305, &sample, ctx).pressed, command::STEP_RIGHT);
}

#[test]
fn the_along_step_mode_reads_the_vertical_axis_with_its_signs() {
    let ctx = PlayContext {
        modes: StepModes {
            across: false,
            along: true,
        },
        ..play_ctx()
    };
    let mut play = primed_play();
    let frame = *play.update(4, &pad().with_analog(Id::Move, Vec2::new(0.0, 0.5)), ctx);
    assert_eq!(frame.step, command::STEP_FORWARD);
    let frame = *play.update(5, &pad().with_analog(Id::Move, Vec2::new(0.0, -0.5)), ctx);
    assert_eq!(frame.step, command::STEP_BACK);
    let frame = *play.update(6, &pad().with_analog(Id::Move, Vec2::new(0.0, 0.1)), ctx);
    assert_eq!(frame.step, 0);
}

#[test]
fn the_power_bar_adds_its_second_command_after_500_ms_held() {
    let powers = pad().with_digital(Id::Powers, true);
    let mut play = primed_play();
    play.update(100, &powers, play_ctx());
    assert_eq!(play.commands() & command::POWER_BAR, command::POWER_BAR);
    assert_eq!(play.update(599, &powers, play_ctx()).pressed, 0);
    assert_eq!(play.update(600, &powers, play_ctx()).pressed, 0);
    assert_eq!(
        play.update(601, &powers, play_ctx()).pressed & command::POWER_HOLD,
        command::POWER_HOLD
    );
}

#[test]
fn the_interact_and_combat_sets_read_their_own_actions() {
    let inventory = pad()
        .with_digital(Id::Inventory, true)
        .with_digital(Id::Skills, true);
    let mut play = primed_play();
    let frame = *play.update(4, &inventory, play_ctx());
    assert_eq!(frame.pressed & command::INVENTORY, command::INVENTORY);
    assert_eq!(frame.pressed & command::SKILLS, command::SKILLS);
    let combat = pad()
        .with_digital(Id::Alternate, true)
        .with_digital(Id::Crouch, true)
        .with_digital(Id::Inventory, true);
    let ctx = PlayContext {
        variant: Variant::Combat,
        ..play_ctx()
    };
    // The set switch primes the held keys for three milliseconds first.
    play.update(400, &combat, ctx);
    let frame = *play.update(403, &combat, ctx);
    assert!(frame.alternate);
    assert!(frame.crouch);
    assert_eq!(frame.pressed & command::INVENTORY, 0);
    play.update(404, &pad(), ctx);
    play.update(405, &inventory, play_ctx());
    let frame = *play.update(408, &inventory, play_ctx());
    assert!(!frame.alternate);
    assert!(!frame.crouch);
    assert_eq!(frame.pressed & command::INVENTORY, 0);
    let frame = *play.update(409, &inventory, play_ctx());
    assert_eq!(frame.pressed & command::INVENTORY, 0);
    assert_eq!(frame.pressed & command::SKILLS, 0);
    play.update(410, &pad(), play_ctx());
    let frame = *play.update(411, &inventory, play_ctx());
    assert_eq!(frame.pressed & command::INVENTORY, command::INVENTORY);
    assert_eq!(frame.pressed & command::SKILLS, command::SKILLS);
}

#[test]
fn look_and_camera_mode_are_held_bits_while_the_others_are_edges() {
    let held = pad()
        .with_digital(Id::Look, true)
        .with_digital(Id::CameraMode, true);
    let mut play = primed_play();
    play.update(4, &held, play_ctx());
    assert_eq!(
        play.commands() & command::LOOK,
        command::LOOK,
        "the look bit is held"
    );
    play.update(5, &held, play_ctx());
    assert_eq!(play.commands() & command::CAMERA_MODE, command::CAMERA_MODE);
    play.update(6, &pad(), play_ctx());
    assert_eq!(play.commands() & command::LOOK, 0);
    assert_eq!(play.commands() & command::CAMERA_MODE, 0);
}

#[test]
fn a_gameplay_set_switch_primes_the_held_buttons() {
    let combat = pad()
        .with_digital(Id::Combat, true)
        .with_digital(Id::Alternate, true);
    let mut play = primed_play();
    let ctx = PlayContext {
        variant: Variant::Combat,
        ..play_ctx()
    };
    play.update(100, &combat, ctx);
    assert_eq!(play.update(103, &combat, ctx).pressed & command::ATTACK, 0);
    // Released and pressed after the priming, it is a press.
    play.update(104, &pad(), ctx);
    let frame = *play.update(105, &combat, ctx);
    assert_eq!(frame.pressed & command::ATTACK, command::ATTACK);
}

#[test]
fn the_gamepad_reader_ignores_input_until_three_milliseconds_after_a_set_change() {
    let sprint = pad().with_digital(Id::Sprint, true);
    let mut play = PlayPad::default();
    let frame = *play.update(0, &sprint, play_ctx());
    assert!(!frame.sprint);
    let frame = *play.update(2, &sprint, play_ctx());
    assert!(!frame.sprint);
    let frame = *play.update(3, &sprint, play_ctx());
    assert!(frame.sprint);
}

#[test]
fn rumble_rises_on_a_hit_decays_by_six_tenths_and_makes_16_bit_motor_words() {
    let mut play = PlayPad::default();
    play.rumble.add(0.5);
    play.rumble.add(0.8);
    assert_eq!(play.rumble.first, 1.0);
    assert_eq!(play.rumble.motors(), (65535, 65535));
    play.rumble = Rumble {
        first: 1.0,
        second: 1.0,
    };
    play.rumble.decay();
    assert!(near(play.rumble.first, 0.6));
    assert_eq!(play.rumble.motors(), (39321, 39321));
    play.rumble.decay();
    assert!(near(play.rumble.first, 0.36));
    assert_eq!(play.rumble.motors(), (23593, 23593));
}

#[test]
fn the_pad_status_needs_a_connected_pad_and_controller_mode() {
    let mut status = PadStatus::default();
    assert!(!status.active());
    status.connected = true;
    assert!(!status.active());
    status.controller = true;
    assert!(status.active());
}

#[test]
fn the_default_table_reads_bevys_buttons_and_sticks() {
    let mut pad = Gamepad::default();
    pad.digital_mut().press(GamepadButton::RightTrigger2);
    pad.digital_mut().press(GamepadButton::South);
    pad.digital_mut().press(GamepadButton::RightTrigger);
    pad.analog_mut().set(GamepadAxis::LeftStickX, 0.5);
    pad.analog_mut().set(GamepadAxis::LeftStickY, -0.25);
    pad.analog_mut().set(GamepadAxis::RightStickY, 0.75);
    let sample = sample_pad(&pad);
    assert!(sample.connected);
    assert!(sample.digital(Id::Combat));
    assert!(sample.digital(Id::Select));
    assert!(sample.digital(Id::ZoomIn));
    assert_eq!(sample.analog(Id::Move), Vec2::new(0.5, -0.25));
    assert_eq!(sample.analog(Id::Camera), Vec2::new(0.0, 0.75));
    // An unbound action is not pressed.
    assert!(!sample.digital(Id::Skills));
}

#[test]
fn the_menu_exit_hook_takes_the_exit_key_instead_of_the_normal_dispatch() {
    let exit = pad().with_digital(Id::MenuExit, true);
    let ctx = MenuContext {
        exit_hook: true,
        ..menu_ctx()
    };
    let mut menu = MenuPad::default();
    assert_eq!(menu.frame(16, &exit, ctx).key, gui_key::BACK);
    // The hook ran; a held exit does not dispatch again.
    assert_eq!(menu.frame(17, &exit, ctx).key, 0);
    assert_eq!(menu.frame(18, &pad(), ctx).key, 0);
    assert_eq!(menu.frame(19, &exit, ctx).key, gui_key::BACK);
}

#[test]
fn a_pad_sample_carries_each_actions_data() {
    let sample = pad()
        .with_digital(Id::Combat, true)
        .with_analog(Id::Camera, Vec2::new(-1.0, 0.5));
    assert!(sample.digital(Id::Combat));
    assert_eq!(sample.analog(Id::Camera), Vec2::new(-1.0, 0.5));
    assert!(!sample.digital(Id::MenuNav));
}
