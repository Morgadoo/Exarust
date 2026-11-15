use super::*;
use crate::gamepad::{ActiveSet, GamepadPlugin, ScriptedPad, Variant};
use crate::menu_pages::{Page, game_mode};
use crate::menu_screen::{MenuScreen, MenuScreenPlugin};
use exa_ui::gui_draw::GuiFrames;

#[test]
fn an_unanswered_word_goes_back_picks_or_reaches_the_control() {
    // Any other answer keeps the word.
    assert_eq!(fallback(0, 0x100, false, true), Fallback::Answered);
    assert_eq!(fallback(0x100, 0x100, false, true), Fallback::Answered);
    // A control that takes the keys gets the whole word.
    assert_eq!(
        fallback(UNANSWERED, 0x8011, true, true),
        Fallback::Control(0x8011)
    );
    // MenuBak closes only a form that asks for it.
    assert_eq!(fallback(UNANSWERED, 0x200, false, true), Fallback::Back);
    assert_eq!(fallback(UNANSWERED, 0x200, false, false), Fallback::Answered);
    // A navigation step picks; a scroll step or MenuSel reaches the
    // control with only its select, back, scroll-step, direction and
    // second analog bits (`& 0xf322`: the 0x10 bit goes).
    assert_eq!(fallback(UNANSWERED, 0x4011, false, false), Fallback::Pick);
    assert_eq!(
        fallback(UNANSWERED, 0x8032, false, false),
        Fallback::Control(0x8022)
    );
    assert_eq!(
        fallback(UNANSWERED, 0x100, false, false),
        Fallback::Control(0x100)
    );
}

#[test]
fn the_dispatch_hands_the_word_to_the_top_form_only() {
    let mut pad = PadMenu::default();
    let frame = MenuFrame {
        key: 0x8011,
        exit_hook: false,
        cursor: Vec2::new(0.0, 2.0),
        scroll_cursor: Vec2::ZERO,
    };
    pad.dispatch(frame, Some(PadForm::Manual.id()), true);
    assert!(pad.gui_gamepad && pad.pad_mode);
    assert_eq!(pad.take(PadForm::MainMenu), None);
    let key = pad.take(PadForm::Manual).unwrap();
    assert_eq!(key.key, 0x8011);
    assert_eq!(key.cursor, Vec2::new(0.0, 2.0));
    assert_eq!(pad.take(PadForm::Manual), None);
    // The exit hook's frame carries no word.
    pad.dispatch(
        MenuFrame {
            exit_hook: true,
            ..MenuFrame::default()
        },
        Some(PadForm::Manual.id()),
        true,
    );
    assert!(pad.exit);
    assert_eq!(pad.peek(), None);
}

#[test]
fn pushes_and_pops_switch_the_sets_and_prime_their_readers() {
    let mut stack = FocusStack::default();
    let mut registry = SetRegistry::default();
    let mut menu = MenuPad::default();
    let mut play = PlayPad::default();
    let mut pad = PadMenu {
        pad_mode: true,
        ..PadMenu::default()
    };
    let mut sets = FocusSets {
        now_ms: 100,
        registry: &mut registry,
        menu: &mut menu,
        play: &mut play,
    };
    push_form(&mut stack, &mut pad, &mut sets, PadForm::MainMenu);
    push_form(&mut stack, &mut pad, &mut sets, PadForm::Manual);
    assert_eq!(sets.registry.active(), ActiveSet::Menus);
    assert_eq!(stack.depth(), 2);
    assert_eq!(stack.entries()[1].flags, 0x4000);
    assert!(pad.gui_gamepad);
    assert!(pad.take_focus_request(PadForm::MainMenu));
    pop_form(&mut stack, &mut pad, &mut sets, PadForm::Manual);
    assert_eq!(sets.registry.active(), ActiveSet::Menus);
    assert!(pad.gui_gamepad);
    pop_form(&mut stack, &mut pad, &mut sets, PadForm::MainMenu);
    assert!(!stack.focused());
    assert!(!pad.gui_gamepad);
    assert_eq!(
        sets.registry.active(),
        ActiveSet::Gameplay {
            index: 0,
            variant: Variant::Interact
        }
    );
    // A form not on the stack pops nothing.
    pop_form(&mut stack, &mut pad, &mut sets, PadForm::Skills);
    assert_eq!(stack.depth(), 0);
}

/// A headless app with the menu screen and the pad, the clock stepping
/// 10 ms a frame, and a scripted pad.
fn menu_app(script: &str) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(10),
        ))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<GuiFrames>()
        .add_plugins((MenuScreenPlugin, GamepadPlugin))
        .insert_resource(ScriptedPad::parse(script).unwrap());
    app
}

fn open_in_game(app: &mut App) {
    let mut menu = app.world_mut().resource_mut::<MenuScreen>();
    menu.open_at(Page::InGame);
    menu.game = game_mode::MENU;
}

/// The menu's show pushes it: the menus become the active set, the
/// gameplay actions read nothing, and the navigation stick's steps and
/// MenuSel reach the menu's key slot (the selection moves; CONTINUE runs
/// and closes the menu). The close pops it and the gameplay layer comes
/// back.
#[test]
fn the_pad_drives_the_open_menu_and_hands_the_game_back() {
    let mut app = menu_app("menu-down@4,talk@5,menu-up@8,menu-down@11,menu-up@14,menu-select@17");
    open_in_game(&mut app);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<SetRegistry>().active(),
        ActiveSet::Menus
    );
    assert_eq!(app.world().resource::<FocusStack>().depth(), 1);
    assert_eq!(app.world().resource::<MenuScreen>().selected, 1);
    app.update();
    // The gameplay set is not active: Talk reads nothing.
    let play = app.world().resource::<PlayPad>();
    assert_eq!(play.frame().pressed, 0);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<MenuScreen>().selected, 0);
    for _ in 0..9 {
        app.update();
    }
    assert!(!app.world().resource::<MenuScreen>().open);
    app.update();
    assert!(!app.world().resource::<FocusStack>().focused());
    assert!(matches!(
        app.world().resource::<SetRegistry>().active(),
        ActiveSet::Gameplay { .. }
    ));
}

/// MenuExit runs the exit hook, the menu's toggle: the open menu closes.
#[test]
fn menu_exit_toggles_the_menu() {
    let mut app = menu_app("menu-exit@5");
    open_in_game(&mut app);
    for _ in 0..4 {
        app.update();
    }
    assert!(app.world().resource::<MenuScreen>().open);
    app.update();
    assert!(!app.world().resource::<MenuScreen>().open);
}

/// The menu app with the Settings form, a window and the configuration,
/// the form opened from the menu's row.
fn settings_app(script: &str) -> App {
    use crate::hw_config::HardwareConfig;
    use crate::menu_pages::MenuCommand;
    use crate::menu_screen::MenuChoice;
    use crate::settings_form::{SettingsForm, SettingsFormPlugin};
    let mut app = menu_app(script);
    app.insert_resource(HardwareConfig::new(
        exa_formats::hw_config::Options::default(),
    ))
    .add_plugins(SettingsFormPlugin);
    // A frame of the pad first: the controller mode is known at the show.
    app.update();
    app.world_mut().spawn((
        Window {
            resolution: bevy::window::WindowResolution::new(1280, 720),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.world_mut().resource_mut::<MenuScreen>().open();
    app.world_mut()
        .write_message(MenuChoice(MenuCommand::OpenSettings));
    app.update();
    assert!(app.world().resource::<SettingsForm>().is_open());
    app
}

/// The Settings form's show pushes it over the menu with its first
/// control focused; a navigation step picks the next record below it and
/// MenuBak takes the form's back slot, which closes it (Cancel) and pops
/// it, leaving the menu on top.
#[test]
fn the_pad_picks_through_the_settings_form_and_backs_out() {
    use crate::settings_form::SettingsForm;
    let mut app = settings_app("menu-down@8,menu-back@14");
    for _ in 0..3 {
        app.update();
    }
    let stack = app.world().resource::<FocusStack>();
    assert_eq!(stack.depth(), 2);
    assert_eq!(
        stack.entries().last().map(|e| e.form),
        Some(PadForm::Settings.id())
    );
    let first = app
        .world()
        .resource::<SettingsForm>()
        .form
        .as_ref()
        .unwrap()
        .pad_focus;
    assert_eq!(first, Some((exa_ui::settings::Part::Main, 0)));
    for _ in 0..5 {
        app.update();
    }
    let picked = app
        .world()
        .resource::<SettingsForm>()
        .form
        .as_ref()
        .unwrap()
        .pad_focus;
    assert!(picked.is_some() && picked != first, "picked {picked:?}");
    for _ in 0..6 {
        app.update();
    }
    assert!(!app.world().resource::<SettingsForm>().is_open());
    app.update();
    let stack = app.world().resource::<FocusStack>();
    assert_eq!(stack.depth(), 1);
    assert!(app.world().resource::<MenuScreen>().open);
}
