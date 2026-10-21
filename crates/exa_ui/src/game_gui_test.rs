use super::*;

#[test]
fn the_game_screens() {
    assert_eq!(GameScreen::Inventory.tag(), Some(0xce1c_2010));
    assert_eq!(GameScreen::Loot.tag(), Some(0xce1c_2040));
    assert_eq!(GameScreen::Journal.tag(), Some(0xce1c_2030));
    assert_eq!(GameScreen::TechniqueSelector.tag(), None);
    assert_eq!(GameScreen::Inventory.size(), Some((244, 644)));
    assert_eq!(GameScreen::Journal.size(), Some((244, 518)));
    assert_eq!(GameScreen::TechniqueSelector.size(), Some((332, 112)));
    assert_eq!(GameScreen::Loot.size(), None);
    assert_eq!(
        (inventory_height(true, false), inventory_height(true, true)),
        (628, 590)
    );
}

#[test]
fn the_open_container_registry() {
    let mut open = OpenContainers::default();
    open.open(11);
    open.open(22);
    assert!(open.is_open(11) && open.is_open(22) && !open.is_open(33));
    open.close(11);
    open.open(33);
    assert_eq!(open.ids().collect::<Vec<_>>(), vec![33, 22]);
    for id in 1..=9 {
        open.open(id);
    }
    // Eight slots: the ninth open in a full registry is dropped.
    assert_eq!(open.ids().count(), OPEN_CONTAINERS);
    assert!(!open.is_open(9));
    assert_eq!(tagged_container(5), 0x2000_0005);
}

fn inventory() -> InventoryInput {
    InventoryInput {
        player_ready: true,
        player_mode_ok: true,
        open: false,
        filling: false,
        active: false,
        toggle: true,
    }
}

#[test]
fn the_show_rules() {
    assert_eq!(inventory_screen(inventory()), ScreenAction::Opened);
    let open = InventoryInput {
        open: true,
        ..inventory()
    };
    assert_eq!(inventory_screen(open), ScreenAction::Focused);
    assert_eq!(
        inventory_screen(InventoryInput {
            active: true,
            ..open
        }),
        ScreenAction::Closed
    );
    assert_eq!(
        inventory_screen(InventoryInput {
            active: true,
            toggle: false,
            ..open
        }),
        ScreenAction::Refused
    );
    assert_eq!(
        inventory_screen(InventoryInput {
            filling: true,
            ..open
        }),
        ScreenAction::Refused
    );
    assert_eq!(
        inventory_screen(InventoryInput {
            player_mode_ok: false,
            ..inventory()
        }),
        ScreenAction::Refused
    );
    assert_eq!(loot_screen(0x300, false), ScreenAction::Refused);
    assert_eq!(loot_screen(0x1_0200, true), ScreenAction::Focused);
    assert_eq!(loot_screen(0x400, false), ScreenAction::Opened);
    let journal = JournalInput {
        player_ready: true,
        player_mode_ok: true,
        open: true,
        active: true,
    };
    assert_eq!(journal_screen(journal), ScreenAction::Closed);
    assert_eq!(
        journal_screen(JournalInput {
            active: false,
            ..journal
        }),
        ScreenAction::Focused
    );
    assert_eq!(
        journal_screen(JournalInput {
            open: false,
            ..journal
        }),
        ScreenAction::Opened
    );
    assert_eq!(
        journal_screen(JournalInput {
            player_ready: false,
            ..journal
        }),
        ScreenAction::Refused
    );
}

#[test]
fn the_inventory_place_and_page() {
    // The player's own goes to its remembered place, another character's
    // to the right edge.
    assert_eq!(
        inventory_place(true, Some((10, 20)), 1600, 244, 4, (8, 8)),
        (10, 20)
    );
    assert_eq!(
        inventory_place(true, None, 1600, 244, 4, INVENTORY_PLACE),
        (8, 8)
    );
    assert_eq!(
        inventory_place(false, Some((10, 20)), 1600, 244, 4, (8, 8)),
        (1340, 8)
    );
    assert_eq!(inventory_tab(0), 2);
    assert_eq!(inventory_tab(0x0100_0000), 3);
    assert_eq!(crystal_item(&[(5, 1), (500, 7), (431, 2)]), (500, 7));
    assert_eq!(crystal_item(&[(5, 1)]), (0, 0));
    assert!(rescales_view(0x40) && !rescales_view(0x80));
}

#[test]
fn the_journal_entries() {
    let mut journal = Journal::default();
    assert_eq!(journal.titles(), vec!["New Entry"]);
    journal.add_entry("first text");
    assert_eq!((journal.entries.len(), journal.current), (2, 1));
    assert_eq!(journal.entries[0].text, "first text");
    // Picking a row stores the text box into the selected entry first.
    journal.list_changed(false, "", 1, "second text");
    assert_eq!(
        (journal.row(), journal.entries[1].text.as_str()),
        (1, "second text")
    );
    journal.list_changed(true, "Day one", 0, "");
    assert_eq!(journal.titles(), vec!["Day one", "New Entry"]);
    // Deleting the first keeps the selection on the entry that moved down.
    journal.delete_entry();
    assert_eq!((journal.entries.len(), journal.current), (1, 0));
    assert_eq!(journal.entries[0].text, "second text");
    // The last one is reset, not removed.
    journal.delete_entry();
    assert_eq!(
        journal.entries,
        vec![JournalEntry {
            title: NEW_ENTRY.into(),
            text: String::new()
        }]
    );
    journal.add_entry("a");
    journal.add_entry("b");
    journal.delete_entry();
    assert_eq!((journal.entries.len(), journal.current), (2, 1));
    assert_eq!(JOURNAL_TEXT.width(), 228);
    assert_eq!(JOURNAL_TEXT.height(), 412);
}

#[test]
fn the_power_slot() {
    let slot = PowerSlot {
        icon: 7,
        count: 3,
        kind: 0,
        skill: 2,
        state: 0,
    };
    let a = slot.dispatch(0x20, 0, false, true, 9);
    assert_eq!((a.result, a.hide_tooltip), (3, true));
    let counted = PowerSlot {
        state: 0x1_0000,
        ..slot
    };
    assert_eq!(counted.dispatch(0x20, 0, false, true, 9).result, 0x103);
    assert!(!counted.dispatch(0x20, 0, false, true, 9).hide_tooltip);
    let a = slot.dispatch(0x40, 0x2000, false, false, 9);
    assert_eq!((a.result, a.open_selector), (3, Some((2, 3))));
    assert_eq!(slot.dispatch(0x40, 0, false, false, 9).result, 0x40);
    let one = PowerSlot { kind: 1, ..slot };
    assert_eq!(one.dispatch(0x40, 0x2000, false, false, 9).result, 0);
    let off = PowerSlot {
        state: 0x10,
        ..slot
    };
    assert_eq!(off.dispatch(0x40, 0x2000, false, false, 9).result, 0x40);
    let empty = PowerSlot { icon: 0, ..slot };
    assert_eq!(empty.dispatch(0x40, 0x2000, false, false, 9).result, 0x40);
    assert!(empty.empty() && !slot.empty());
    assert!(slot.dispatch(0x50, 0x1_0000, true, false, 9).toggle);
    assert!(!slot.dispatch(0x50, 0x1_0000, false, false, 9).toggle);
    assert_eq!(slot.dispatch(0x8000, 0, false, false, 9).result, 9);
    assert_eq!(slot.dispatch(1, 0, false, false, 9), SlotAnswer::default());
}

#[test]
fn the_counter() {
    let mut counter = Counter {
        segments: 3,
        ..Counter::default()
    };
    // Block round(100 / 4 − 1) = 24; fills 21 wide, 25 apart.
    counter.set(0.5);
    assert_eq!(counter.text(), "50.0%");
    let w = |v: &[CounterBlock]| v.iter().map(|b| (b.x, b.width)).collect::<Vec<_>>();
    assert_eq!(w(&counter.fills(0, 100)), vec![(2, 21), (27, 21)]);
    counter.set(0.6);
    // The third share is 0.4: round(8.4) = 8.
    assert_eq!(w(&counter.fills(0, 100)), vec![(2, 21), (27, 21), (52, 8)]);
    // The backs step one pixel each.
    assert_eq!(
        w(&counter.backs(0, 100)),
        vec![(1, 23), (2, 23), (3, 23), (4, 23)]
    );
    counter.set(-0.2);
    assert_eq!(counter.value, 0.0);
    counter.set(-0.0);
    assert!(counter.value.is_sign_negative());
    counter.set(f32::NAN);
    assert!(counter.value.is_nan());
    // A NaN share does not end the bar; its fill rounds to nothing.
    assert_eq!(
        w(&counter.fills(0, 100)),
        vec![(2, 0), (27, 0), (52, 0), (77, 0)]
    );
    let one = Counter {
        value: 1.0,
        segments: 0,
    };
    assert_eq!(w(&one.fills(0, 100)), vec![(2, 96)]);
    assert_eq!(bar_control_place(10, 20, 28), (12, 21, 24));
    assert_eq!(HUD_CONTROLS[0], (0xac3, 40, 40));
}

#[test]
fn the_status_marker_and_ready_alert() {
    assert_eq!(status_pulse(0, 0), (1.0, 0.6));
    assert_eq!(status_pulse(1000, 0).0, 0.0);
    let (e, glow) = status_pulse(1500, 0);
    assert_eq!(e, 0.5);
    assert!((glow - 0.3).abs() < 1e-7);
    // A clock behind the start gives t = 1.5, which eases to 0.
    assert_eq!(status_pulse(0, 500).0, 0.0);
    assert_eq!(status_place(1600, 900, 5), (1065, 860));
    assert_eq!(ready_place(1600, 900, 5), (500, 860));
    assert_eq!(status_glow((100, 200), 4), [112, 212, 120, 220]);
    assert!(!status_marker(&[]));
    assert!(status_marker(&[0x1000]));
    assert!(!status_marker(&[0, 0x2000]));
    let mut skills = [Skill::default(); 6];
    assert!(ready_alert(&skills));
    skills[0] = Skill {
        points: 5000,
        techniques: [3, 4, 0, 0, 0, 0],
    };
    // Level 1 reads two bytes: two chosen is ahead.
    assert!(techniques_ahead(&skills));
    assert!(!ready_alert(&skills));
    skills[0].techniques = [3, 0, 0, 0, 0, 0];
    assert!(!techniques_ahead(&skills));
    for skill in skills.iter_mut().take(3) {
        skill.points = 25_000;
        skill.techniques = [0; 6];
    }
    assert!(!ready_alert(&skills));
}

#[test]
fn the_level_curtain() {
    let mut curtain = LevelCurtain {
        state: 1,
        start: 1000,
    };
    // Half-way: 127.5 rounds to the even 128.
    assert_eq!(curtain.step(2500), (128, 0.5, false));
    assert_eq!(curtain.step(4001), (255, 0.0, true));
    assert_eq!(curtain.state, 2);
    assert_eq!(curtain.step(5000), (255, 0.0, false));
    assert!(curtain.reset());
    assert!(!curtain.reset());
    // A clock behind the start: −85 keeps its low byte.
    let (alpha, bright, _) = LevelCurtain {
        state: 1,
        start: 1000,
    }
    .step(0);
    assert_eq!(alpha, 171);
    assert!((bright - 4.0 / 3.0).abs() < 1e-6);
}

#[test]
fn the_compass_heartbeat_and_cursor() {
    assert_eq!(compass_extent(4), 48.0);
    assert_eq!(compass_centre(4, 900), (56.0, 844.0));
    assert_eq!(
        (compass_alpha(0.0), compass_alpha(1.0), compass_alpha(0.5)),
        (192, 0, 96)
    );
    assert_eq!(heartbeat_fade_gain(0, 0), Some(0.3));
    assert!((heartbeat_fade_gain(500, 0).unwrap() - 0.15).abs() < 1e-7);
    assert_eq!(heartbeat_fade_gain(1000, 0), None);
    assert_eq!(bars_hold_until(u32::MAX, 1), 0);
    assert_eq!(menu_cursor_size(1080, 1.0), 32);
    assert_eq!(menu_cursor_size(2160, 1.0), 64);
    assert_eq!(menu_cursor_size(1080, 4.0), 128);
    assert_eq!(menu_cursor_hotspot(32), (6, 3));
    assert_eq!(menu_cursor_hotspot(128), (23, 10));
}

#[test]
fn the_hud_layers() {
    let full = HudFrame {
        heartbeat: true,
        hook: true,
        enabled: true,
        player: true,
        curtain: true,
        compass: true,
        bars_alpha: 1.0,
        gauge_alpha: 0.5,
        message: true,
    };
    use HudLayer::*;
    assert_eq!(
        hud_layers(full),
        vec![
            Heartbeat,
            Hook,
            Curtain,
            Compass,
            StatusMarker,
            MarkerHook,
            Bars,
            Gauge,
            Dialogue,
            Message
        ]
    );
    assert_eq!(
        hud_layers(HudFrame {
            player: false,
            ..full
        }),
        vec![Heartbeat, Hook]
    );
    assert_eq!(hud_layers(HudFrame::default()), vec![Dialogue]);
    assert_eq!(
        hud_layers(HudFrame {
            gauge_alpha: 0.0,
            heartbeat: false,
            hook: false,
            curtain: false,
            compass: false,
            message: false,
            ..full
        }),
        vec![StatusMarker, MarkerHook, Bars, Dialogue]
    );
}

#[test]
fn the_save_and_close_scopes() {
    assert_eq!(
        save_action(CONTAINER_SCREEN, false),
        Some(SaveAction::Container)
    );
    assert_eq!(
        save_action(JOURNAL_SCREEN, false),
        Some(SaveAction::Journal)
    );
    assert_eq!(
        save_action(INVENTORY_SCREEN, true),
        Some(SaveAction::Inventory)
    );
    assert_eq!(save_action(INVENTORY_SCREEN, false), None);
    assert_eq!(save_action(LOOT_SCREEN, true), Some(SaveAction::Loot));
    assert_eq!(save_action(0x1234, true), None);
    let tags = [CONTAINER_SCREEN, JOURNAL_SCREEN, 0xce10_0001, 0x1234];
    assert_eq!(forms_with_tag(&tags, CONTAINER_FAMILY, true), vec![0, 1]);
    assert_eq!(
        forms_with_tag(&tags, GAME_SCREEN_FAMILY, false),
        vec![2, 1, 0]
    );
    // The family's mask also covers the container family.
    assert!(has_game_screen(&[JOURNAL_SCREEN]));
    assert!(!has_game_screen(&[0x1234]));
    assert_eq!(
        loot_windows_to_close(&[
            (LOOT_SCREEN, 0, 0xa00),
            (LOOT_SCREEN, 0x80000, 0xa00),
            (INVENTORY_SCREEN, 0, 0),
            (LOOT_SCREEN, 0, 0),
        ]),
        vec![1, 3]
    );
}

#[test]
fn the_technique_selector_opening() {
    let place = selector_place(100, 200, 40, 40);
    assert_eq!(
        (
            place.desired_x,
            place.fallback_x,
            place.desired_y,
            place.fallback_bottom
        ),
        (140, 100, 201, 239)
    );
    assert!(selector_locked(true, 2, 10_000));
    assert!(!selector_locked(true, 3, 10_000));
    assert!(!selector_locked(false, 1, 10_000));
    let mut requirements = [[0; 2]; 8];
    requirements[4] = [3, 0];
    requirements[5] = [5, 0];
    requirements[6] = [0, 2];
    let input = SelectorInput {
        locked: false,
        skill: Skill {
            points: 10_000,
            techniques: [3, 0, 0, 0, 0, 0],
        },
        requirements,
        count: 1,
    };
    let (rows, forget) = selector_rows(input);
    assert_eq!(rows, [0, 0, 0x10, 0, 0, 0x10, 0x10, 0, 0]);
    assert!(forget);
    let (rows, forget) = selector_rows(SelectorInput {
        locked: true,
        count: 0,
        ..input
    });
    assert_eq!(rows, [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0]);
    assert!(!forget);
}

#[test]
fn the_preview_bars_and_journal_flag() {
    let mut flag = false;
    assert_eq!(preview_bars_toggle(&mut flag, true), Some(true));
    assert_eq!(preview_bars_toggle(&mut flag, true), None);
    assert_eq!(preview_bars_toggle(&mut flag, false), Some(false));
    assert_eq!(preview_shares(65535, 0), (1.0, 0.0));
    assert!((preview_shares(32768, 16384).0 - 0.500_007_6).abs() < 1e-6);
    assert_eq!(
        (journal_controls_flag(true), journal_controls_flag(false)),
        (0, 1)
    );
}

#[test]
fn the_journal_keeps_its_text_and_place_when_it_closes() {
    let mut journal = Journal::default();
    journal.close((120, 40), "a note");
    assert_eq!(journal.place, Some((120, 40)));
    assert_eq!(journal.entries[0].text, "a note");
}
