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
    // Exactly a quarter steps nowhere but is past the analog tenth.
    let edge = pad().with_analog(Id::MenuNav, Vec2::new(0.0, 0.5));
    assert_eq!(menu.frame(16, &edge, menu_ctx()).key, gui_key::ANALOG);
    let over = pad().with_analog(Id::MenuNav, Vec2::new(0.0, 0.5001));
    assert_eq!(
        menu.frame(32, &over, menu_ctx()).key,
        gui_key::UP | gui_key::ANALOG
    );
}

#[test]
fn a_stick_past_a_tenth_sends_its_analog_bit_and_scaled_cursor_every_frame() {
    // 0.3 squared is 0.09: inside the analog tenth, the cursor is the raw
    // stick (y down) and the scroll stick is read.
    let mut menu = MenuPad::default();
    let small = pad()
        .with_analog(Id::MenuNav, Vec2::new(0.3, 0.0))
        .with_analog(Id::MenuScreen, Vec2::new(0.0, 0.4));
    let frame = menu.frame(32, &small, menu_ctx());
    assert_eq!(frame.key, gui_key::SCROLL_ANALOG);
    assert_eq!(frame.cursor, Vec2::new(0.3, 0.0));
    // 32 ms at a sixteenth is a factor of 2.
    assert_eq!(frame.scroll_cursor, Vec2::new(0.0, -0.8));
    // 0.4 squared is 0.16: the bit and the scaled cursor, no direction, and
    // the scroll stick is not read, so its cursor stays.
    let middle = pad().with_analog(Id::MenuNav, Vec2::new(0.4, 0.0));
    let frame = menu.frame(48, &middle, menu_ctx());
    assert_eq!(frame.key, gui_key::ANALOG);
    assert_eq!(frame.cursor, Vec2::new(0.4, 0.0));
    assert_eq!(frame.scroll_cursor, Vec2::new(0.0, -0.8));
}

#[test]
fn a_nan_stick_is_vertical_and_down_or_left() {
    assert_eq!(
        MenuPad::direction(Vec2::new(f32::NAN, 0.0), false),
        gui_key::DOWN
    );
    assert_eq!(
        MenuPad::direction(Vec2::new(0.6, f32::NAN), false),
        gui_key::DOWN
    );
    assert_eq!(
        MenuPad::direction(Vec2::new(-0.6, 0.0), false),
        gui_key::LEFT
    );
    assert_eq!(
        MenuPad::direction(Vec2::new(0.0, 0.0), true),
        gui_key::SCROLL_DOWN
    );
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
    let mut key = |at: i64, sample: &PadSample| menu.frame(at, sample, menu_ctx()).key;
    // Between the steps the pushed stick still sends its analog bit.
    let step = gui_key::UP | gui_key::ANALOG;
    assert_eq!(key(16, &sample), step);
    assert_eq!(key(100, &sample), gui_key::ANALOG);
    assert_eq!(key(416, &sample), gui_key::ANALOG);
    assert_eq!(key(417, &sample), step);
    assert_eq!(key(567, &sample), gui_key::ANALOG);
    assert_eq!(key(568, &sample), step);
    // A stick that returns inside the dead zone resets the first delay.
    assert_eq!(key(600, &pad()), 0);
    assert_eq!(key(700, &sample), step);
    assert_eq!(key(1100, &sample), gui_key::ANALOG);
    assert_eq!(key(1101, &sample), step);
    assert_eq!(key(1251, &sample), gui_key::ANALOG);
    assert_eq!(key(1252, &sample), step);
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
    assert_eq!(menu.frame(100, &sample, ctx).key, gui_key::ANALOG);
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
        pick_control(Vec2::new(0.0, 1.0), IVec2::ZERO, &targets, 4),
        Some(2)
    );
    // Nothing lies below.
    assert_eq!(
        pick_control(Vec2::new(0.0, -1.0), IVec2::ZERO, &targets, 4),
        None
    );
}

#[test]
fn a_pick_needs_a_unit_along_the_axis_and_reaches_eighty_units_sideways() {
    let at = |id: i32, x: i32, y: i32| PickTarget {
        id,
        centre: IVec2::new(x, y),
        reachable: true,
        current: false,
    };
    // Three pixels along is under the unit of 4; four is enough.
    assert_eq!(pick_control(Vec2::Y, IVec2::ZERO, &[at(1, 0, 3)], 4), None);
    assert_eq!(
        pick_control(Vec2::Y, IVec2::ZERO, &[at(1, 0, 4)], 4),
        Some(1)
    );
    // 600 pixels sideways is past 320 at unit 4, inside 640 at unit 8.
    assert_eq!(
        pick_control(Vec2::Y, IVec2::ZERO, &[at(1, 600, 40)], 4),
        None
    );
    assert_eq!(
        pick_control(Vec2::Y, IVec2::ZERO, &[at(1, 600, 40)], 8),
        Some(1)
    );
    // The direction is normalised first: a short cursor picks the same.
    assert_eq!(
        pick_control(Vec2::new(0.0, 0.01), IVec2::ZERO, &[at(1, 0, 4)], 4),
        Some(1)
    );
    // A vertical tie in rounded distance goes to the smaller sideways one.
    let tie = [at(1, 3, 50), at(2, 2, 50)];
    assert_eq!(pick_control(Vec2::Y, IVec2::ZERO, &tie, 4), Some(2));
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
        pick_control(Vec2::new(1.0, 0.0), IVec2::ZERO, &targets, 4),
        Some(1)
    );
    // With only the far-down target left, its stretched offset is under the
    // 0.3 alignment.
    assert_eq!(
        pick_control(Vec2::new(1.0, 0.0), IVec2::ZERO, &targets[2..], 4),
        None
    );
}

#[test]
fn the_focus_stack_moves_duplicates_and_picks_the_sets() {
    let mut stack = FocusStack::default();
    assert!(!stack.focused());
    assert_eq!(
        stack.push(FocusEntry { form: 7, flags: 0 }),
        FocusChange::Menus
    );
    assert_eq!(
        stack.push(FocusEntry {
            form: 8,
            flags: 0x8001
        }),
        FocusChange::Gameplay(0)
    );
    assert_eq!(
        stack.push(FocusEntry {
            form: 9,
            flags: 0x8002
        }),
        FocusChange::Gameplay(1)
    );
    assert_eq!(stack.depth(), 3);
    assert!(stack.contains(8) && !stack.contains(10));
    // A form pushed again moves to the top.
    assert_eq!(
        stack.push(FocusEntry { form: 7, flags: 0 }),
        FocusChange::Menus
    );
    assert_eq!(stack.depth(), 3);
    assert_eq!(stack.entries().last().map(|e| e.form), Some(7));
    assert_eq!(stack.pop(7), FocusChange::Gameplay(1));
    assert_eq!(stack.pop(9), FocusChange::Gameplay(0));
    // The last pop restores the gameplay layer rather than the menus.
    assert_eq!(stack.pop(8), FocusChange::Restore);
    assert!(!stack.focused());
    assert_eq!(stack.pop(8), FocusChange::None);
    for form in 0..9 {
        stack.push(FocusEntry { form, flags: 0 });
    }
    assert_eq!(stack.depth(), FocusStack::LIMIT);
}

#[test]
fn the_camera_stick_has_a_half_dead_zone_vertically_and_a_squared_tenth_horizontally() {
    assert_eq!(look_input(Vec2::new(0.0, 0.4999)), None);
    assert_eq!(look_input(Vec2::new(0.0, 0.5)), Some(Vec2::new(0.0, 5.0)));
    assert_eq!(look_input(Vec2::new(0.0, -0.5)), Some(Vec2::new(0.0, -5.0)));
    assert_eq!(look_input(Vec2::new(0.09, 0.0)), None);
    // 0.1 passes the horizontal dead zone (as a single it is just over the
    // double 0.1) but squares to 0.01, whose square 0.0001 is under the
    // pair threshold: no look.
    assert_eq!(look_input(Vec2::new(0.1, 0.0)), None);
    // Squared, the horizontal axis must then clear the 0.01 pair threshold:
    // 0.2 gives 0.04, whose square 0.0016 is under it; 0.4 gives 0.16.
    assert_eq!(look_input(Vec2::new(0.2, 0.0)), None);
    let look = look_input(Vec2::new(0.4, 0.0)).unwrap();
    assert!(near(look.x, 0.16 * 30.0) && look.y == 0.0, "{look:?}");
    assert_eq!(look_input(Vec2::new(-0.5, 0.0)), Some(Vec2::new(-7.5, 0.0)));
    // A NaN axis passes the horizontal dead zone, as the unordered compare
    // does, and then fails the pair threshold.
    assert_eq!(look_input(Vec2::new(f32::NAN, 0.0)), None);
}

#[test]
fn the_look_turns_at_the_orbit_rate_and_raises_the_pitch_for_up() {
    let look = look_input(Vec2::new(1.0, 1.0)).unwrap();
    assert_eq!(look, Vec2::new(30.0, 10.0));
    // On a 1920-pixel screen the orbit rate is 0.0025 rad a pixel: a full
    // stick turns 0.075 rad of yaw and 0.025 rad of pitch a frame.
    let rate = crate::nav_camera::orbit_per_pixel(1920.0);
    let (yaw, pitch) = look_turn(1.0, 1.0, 0.8, 1.5, look, rate);
    assert!(near(yaw, 1.0 - 0.075), "{yaw}");
    assert!(near(pitch, 1.025), "{pitch}");
    // The turn starts from the held pitch: an asked pitch under the floor
    // counts as the floor.
    let (_, pitch) = look_turn(0.0, 0.5, 0.8, 1.5, Vec2::new(0.0, 4.0), rate);
    assert!(near(pitch, 0.81), "{pitch}");
    // And it is held between the floor and the cap.
    let (_, pitch) = look_turn(0.0, 1.49, 0.8, 1.5, look, rate);
    assert_eq!(pitch, 1.5);
    let (_, pitch) = look_turn(0.0, 0.81, 0.8, 1.5, -look, rate);
    assert_eq!(pitch, 0.8);
}

#[test]
fn a_zoom_frame_moves_a_hundred_and_twentieth_of_a_stop_per_step() {
    // The stop is 240 on 800–2000; ten steps of 1/120 move 20, inwards.
    assert_eq!(zoom_step(1300.0, ZOOM_IN_STEPS, 800.0, 2000.0), 1280.0);
    assert_eq!(zoom_step(1300.0, -ZOOM_IN_STEPS, 800.0, 2000.0), 1320.0);
    assert_eq!(zoom_step(810.0, ZOOM_IN_STEPS, 800.0, 2000.0), 800.0);
    assert_eq!(zoom_step(1990.0, -ZOOM_IN_STEPS, 800.0, 2000.0), 2000.0);
    assert!(zoom_step(f32::NAN, ZOOM_IN_STEPS, 800.0, 2000.0).is_nan());
}

#[test]
fn the_zoom_buttons_are_read_into_the_frame() {
    let mut play = primed_play();
    let sample = pad()
        .with_digital(Id::ZoomIn, true)
        .with_analog(Id::Camera, Vec2::new(0.0, 1.0));
    let frame = *play.update(4, &sample, play_ctx());
    assert!(frame.zoom_in && !frame.zoom_out);
    assert_eq!(frame.look, Some(Vec2::new(0.0, 10.0)));
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
fn the_aim_stick_turns_into_the_camera_frame_or_stays_a_raw_pair() {
    let sample = pad().with_analog(Id::Aim, Vec2::new(0.0, 1.0));
    let mut play = primed_play();
    let frame = *play.update(4, &sample, play_ctx());
    assert_eq!(frame.aim_world, Vec3::Z);
    let ctx = PlayContext {
        raw_aim: true,
        ..play_ctx()
    };
    // The raw pair fills x and y and keeps the last z.
    let sample = pad().with_analog(Id::Aim, Vec2::new(0.5, -0.25));
    let frame = *play.update(5, &sample, ctx);
    assert_eq!(frame.aim_world, Vec3::new(0.5, -0.25, 1.0));
}

#[test]
fn a_step_mode_zeroes_the_move_and_keeps_the_run_flag() {
    let mut play = primed_play();
    let fast = pad().with_analog(Id::Move, Vec2::new(0.0, 1.0));
    assert!(play.update(4, &fast, play_ctx()).move_fast);
    let ctx = PlayContext {
        modes: StepModes {
            power_bar: false,
            dialogue: true,
        },
        ..play_ctx()
    };
    let frame = *play.update(5, &pad(), ctx);
    assert!(frame.move_fast);
    assert_eq!(frame.move_world, Vec3::ZERO);
    assert_eq!(frame.move_raw, Vec2::ZERO);
}

#[test]
fn every_digital_gameplay_action_sets_its_command_bit() {
    let cases = [
        (Id::Combat, command::ATTACK),
        (Id::SwapWeapon, command::SWAP_WEAPON),
        (Id::Inventory, command::INVENTORY),
        (Id::Skills, command::SKILLS),
        (Id::UsePower, command::USE_POWER),
        (Id::Powers, command::POWER_BAR),
        (Id::Select, command::USE),
        (Id::Talk, command::TALK),
        (Id::PauseMenu, command::PAUSE),
    ];
    for (id, bit) in cases {
        let mut play = primed_play();
        let sample = pad().with_digital(id, true);
        let frame = *play.update(4, &sample, play_ctx());
        assert_eq!(frame.pressed & bit, bit, "{id:?}");
        assert_eq!(play.commands() & bit, bit, "{id:?}");
    }
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
fn the_power_bar_step_mode_steps_every_three_hundred_ms() {
    let sample = pad().with_analog(Id::Move, Vec2::new(0.5, 0.0));
    let ctx = PlayContext {
        modes: StepModes {
            power_bar: true,
            dialogue: false,
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
fn the_dialogue_step_mode_reads_the_vertical_axis_with_its_signs() {
    let ctx = PlayContext {
        modes: StepModes {
            power_bar: false,
            dialogue: true,
        },
        ..play_ctx()
    };
    let mut play = primed_play();
    let frame = *play.update(4, &pad().with_analog(Id::Move, Vec2::new(0.0, 0.5)), ctx);
    assert_eq!(frame.step, command::STEP_UP);
    let frame = *play.update(5, &pad().with_analog(Id::Move, Vec2::new(0.0, -0.5)), ctx);
    assert_eq!(frame.step, command::STEP_DOWN);
    // The widened single-precision 0.1 is just over the double 0.1.
    let frame = *play.update(6, &pad().with_analog(Id::Move, Vec2::new(0.0, 0.1)), ctx);
    assert_eq!(frame.step, command::STEP_UP);
    let frame = *play.update(7, &pad().with_analog(Id::Move, Vec2::new(0.0, 0.099)), ctx);
    assert_eq!(frame.step, 0);
}

#[test]
fn a_held_power_bar_asks_for_the_power_tree_after_500_ms() {
    let powers = pad().with_digital(Id::Powers, true);
    let mut play = primed_play();
    play.update(100, &powers, play_ctx());
    assert_eq!(play.commands() & command::POWER_BAR, command::POWER_BAR);
    assert_eq!(play.update(599, &powers, play_ctx()).pressed, 0);
    assert_eq!(play.update(600, &powers, play_ctx()).pressed, 0);
    assert_eq!(
        play.update(601, &powers, play_ctx()).pressed & command::POWER_TREE,
        command::POWER_TREE
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
fn the_set_registry_starts_on_interact_and_switches_with_the_variant() {
    let mut sets = SetRegistry::default();
    assert_eq!(sets.names(), ["Set_Interact", "Set_Combat"]);
    assert_eq!(sets.variant(), Some(Variant::Interact));
    assert_eq!(sets.layer(), Some(1));
    assert!(!sets.activate(1));
    assert!(sets.activate(2));
    assert_eq!(sets.variant(), Some(Variant::Combat));
    assert_eq!(
        sets.active(),
        ActiveSet::Gameplay {
            index: 1,
            variant: Variant::Combat
        }
    );
    assert!(sets.activate(0));
    assert_eq!(sets.active(), ActiveSet::Menus);
    assert_eq!(sets.variant(), None);
    assert!(sets.apply(FocusChange::Gameplay(1)));
    assert_eq!(sets.variant(), Some(Variant::Combat));
    assert!(!sets.apply(FocusChange::None));
    assert_eq!(sets.register("Set_Test"), 3);
}

#[test]
fn the_gameplay_layer_waits_under_a_form_and_comes_back_when_it_leaves() {
    let mut sets = SetRegistry::default();
    let mut stack = FocusStack::default();
    // A form takes the menus.
    assert!(sets.apply(stack.push(FocusEntry { form: 1, flags: 0 })));
    assert_eq!(sets.active(), ActiveSet::Menus);
    // Combat mode under the form selects the combat layer but leaves the
    // menus active; selecting it again is no change.
    assert!(sets.set_variant(Variant::Combat, stack.focused()));
    assert!(!sets.set_variant(Variant::Combat, stack.focused()));
    assert_eq!(sets.active(), ActiveSet::Menus);
    assert_eq!(sets.layer(), Some(2));
    // The form leaves: the combat layer becomes active.
    assert!(sets.apply(stack.pop(1)));
    assert_eq!(sets.variant(), Some(Variant::Combat));
    assert_eq!(sets.layer(), Some(2));
    // With the menus as the layer, a restore changes nothing.
    assert!(sets.select(0, false));
    assert_eq!(sets.active(), ActiveSet::Menus);
    assert!(!sets.apply(FocusChange::Restore));
}

#[test]
fn a_picked_control_receives_the_focus_bit_when_its_form_has_controls() {
    assert_eq!(picked_key(7, false), 7);
    assert_eq!(picked_key(7, true), 0x1_0007);
}

#[test]
fn the_raw_fallback_scales_its_stick_and_marks_the_run() {
    let (move_world, fast) = raw_move(16384, 0, Vec3::X, Vec3::Z);
    assert!((move_world.x - 0.5).abs() < 1e-5, "{move_world:?}");
    assert!(!fast);
    let (_, fast) = raw_move(25000, 25000, Vec3::X, Vec3::Z);
    assert!(fast);
    let (move_world, _) = raw_move(32767, 0, Vec3::X, Vec3::Z);
    assert!((move_world.x - 0.999_969_5).abs() < 1e-5);
    assert!(raw_button(0x40));
    assert!(!raw_button(0xbf));
}

#[test]
fn the_plugin_reads_a_connected_bevy_gamepad_into_the_readers() {
    use bevy::time::TimePlugin;

    let mut app = App::new();
    app.add_plugins((TimePlugin, GamepadPlugin))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(10),
        ));
    let mut pad = Gamepad::default();
    pad.digital_mut().press(GamepadButton::South);
    pad.analog_mut().set(GamepadAxis::LeftStickX, 0.5);
    app.world_mut().spawn((pad, GamepadSettings::default()));
    app.update();
    // The first frame primes the reader; the second reads the held keys.
    app.update();
    let status = *app.world().resource::<PadStatus>();
    assert!(status.connected);
    assert!(status.auto);
    assert!(status.controller);
    let play = app.world().resource::<PlayPad>();
    assert!(play.frame().state & command::USE != 0);
    assert_eq!(play.frame().move_raw, Vec2::new(0.5, 0.0));
    assert_eq!(play.frame().move_world, Vec3::new(0.5, 0.0, 0.0));
}

#[test]
fn pad_activity_counts_a_button_or_a_half_pushed_stick() {
    let pad = Gamepad::default();
    assert!(!pad_activity(&pad));
    let mut pressed = Gamepad::default();
    pressed.digital_mut().press(GamepadButton::South);
    assert!(pad_activity(&pressed));
    let mut stick = Gamepad::default();
    stick.analog_mut().set(GamepadAxis::LeftStickX, 0.51);
    assert!(pad_activity(&stick));
    stick.analog_mut().set(GamepadAxis::LeftStickX, 0.49);
    assert!(!pad_activity(&stick));
    let mut trigger = Gamepad::default();
    trigger.analog_mut().set(GamepadButton::RightTrigger2, 0.6);
    assert!(pad_activity(&trigger));
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
    let exit = pad()
        .with_digital(Id::MenuExit, true)
        .with_analog(Id::MenuNav, Vec2::new(0.5, 0.0));
    let ctx = MenuContext {
        exit_hook: true,
        ..menu_ctx()
    };
    let mut menu = MenuPad::default();
    let frame = menu.frame(16, &exit, ctx);
    assert!(frame.exit_hook);
    assert_eq!(frame.key, 0);
    // The cursor was still read: the hook's frame keeps it.
    assert_eq!(frame.cursor, Vec2::new(0.5, 0.0));
    // A held exit does not run the hook again, and nothing is dispatched.
    let frame = menu.frame(17, &exit, ctx);
    assert!(!frame.exit_hook);
    assert_eq!(frame.key, 0);
    assert!(!menu.frame(18, &pad(), ctx).exit_hook);
    assert!(menu.frame(19, &exit, ctx).exit_hook);
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
