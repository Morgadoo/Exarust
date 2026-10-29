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
    assert_eq!(GameScreen::Loot.size(), Some((244, 440)));
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
    // The backs step block + 1 = 25 each.
    assert_eq!(
        w(&counter.backs(0, 100)),
        vec![(1, 23), (26, 23), (51, 23), (76, 23)]
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
    // The full layout: backs over each segment, the filled parts, and the
    // outline of every filled part wider than two pixels.
    let mut counter = Counter {
        segments: 3,
        ..Counter::default()
    };
    counter.set(0.5);
    let rects = |v: &[CounterRect]| {
        v.iter()
            .map(|r| (r.x, r.y, r.width, r.height))
            .collect::<Vec<_>>()
    };
    let l = counter.layout(10, 20, 100, 16, 0);
    assert_eq!(
        rects(&l.backs),
        vec![
            (11, 20, 23, 16),
            (36, 20, 23, 16),
            (61, 20, 23, 16),
            (86, 20, 23, 16)
        ]
    );
    // Each back's shade: two pixels in, block − 5 wide.
    assert_eq!(rects(&l.shades)[1], (38, 22, 19, 12));
    assert_eq!(rects(&l.fills), vec![(12, 21, 21, 14), (37, 21, 21, 14)]);
    assert_eq!(rects(&l.outlines), vec![(13, 22, 19, 12), (38, 22, 19, 12)]);
    // The record's row inset moves the fills and outlines in.
    let inset = counter.layout(10, 20, 100, 16, 1);
    assert_eq!(rects(&inset.fills)[0], (12, 22, 21, 12));
    assert_eq!(rects(&inset.outlines)[0], (13, 23, 19, 10));
    counter.set(0.6);
    let l = counter.layout(10, 20, 100, 16, 0);
    assert_eq!(
        rects(&l.fills),
        vec![(12, 21, 21, 14), (37, 21, 21, 14), (62, 21, 8, 14)]
    );
    assert_eq!(rects(&l.outlines).last(), Some(&(63, 22, 6, 12)));
    assert_eq!(bar_control_place(10, 20, 28), (12, 21, 24));
    assert_eq!(HUD_CONTROLS[0], (0xac3, 40, 40));
    // The bars' and gauge's frame quads.
    assert_eq!(bar_frame(550, 989, 240, 5), (550, 989, 790, 1007));
    assert_eq!(gauge_frame(680, 969, 240, 10), (680, 968, 920, 982));
}

#[test]
fn the_status_marker_and_ready_alert() {
    assert_eq!(status_pulse(0, 0), (1.0, 0.6));
    assert_eq!(status_pulse(1000, 0).0, 0.0);
    let (e, glow) = status_pulse(1500, 0);
    assert_eq!(e, 0.5);
    assert!((glow - 0.3).abs() < 1e-7);
    // A clock behind the start: the 64-bit difference −500 keeps its sign,
    // t = |1000 + 500| / 1000 = 1.5, which eases to 0.
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
    // A clock behind the start: the 64-bit difference is negative and
    // round(−85) keeps its low byte, 171; the world brightens past 1.
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
    // A clock behind the start: the negative difference raises the gain.
    assert!((heartbeat_fade_gain(0, 500).unwrap() - 0.45).abs() < 1e-6);
    assert_eq!(bars_hold_until(u32::MAX, 1), 0);
    // The CursorSize setting (1 shipped) scales the sprite; never above
    // 128 pixels.
    assert_eq!(menu_cursor_size(1080, 1.0), 32);
    assert_eq!(menu_cursor_size(2160, 1.0), 64);
    assert_eq!(menu_cursor_size(1080, 4.0), 128);
    assert_eq!(menu_cursor_size(2160, 4.0), 128);
    assert_eq!(menu_cursor_hotspot(32), (6, 3));
    assert_eq!(menu_cursor_hotspot(128), (23, 10));
}

#[test]
fn the_skins_compass_install_and_resets() {
    assert_eq!((GUITEX_FILE, GUITEX_STEM), ("guitex.rfi", "guitex"));
    let mut skins = GameSkins::default();
    set_skin_unit(&mut skins, 5);
    assert_eq!(skins.unit, 5);
    assert!(compass_install(true).is_none());
    let install = compass_install(false).unwrap();
    assert_eq!(
        (install.callback, install.texture, install.texture_flags),
        (0x2f3, COMPASS, 0x3f0)
    );
    assert!(install.world_cursor && install.fade_hook);
    assert_eq!(
        shutdown_skin(true, false),
        SkinShutdown {
            close_guitex: true,
            run_hook: false
        }
    );
    // The screen reset releases a fallen curtain's hold and starts the
    // heartbeat's fade at the clock.
    let mut curtain = LevelCurtain { state: 2, start: 0 };
    let mut holds = 1;
    assert_eq!(screen_reset(&mut curtain, &mut holds, true, 700), Some(700));
    assert_eq!((curtain.state, holds), (0, 0));
    assert_eq!(start_heartbeat_fade(false, 700), None);
    // The HUD's fades both end at the reset clock with no opacity.
    let reset = reset_hud_fades(100, Some((0.5, 0.75)));
    assert_eq!(
        (reset.bars.end, reset.bars.opacity, reset.gauge.end),
        (100, 0.0, 100)
    );
    assert_eq!(reset.hold, 0);
    assert_eq!(reset.last, Some((0.5, 0.75)));
    // The bars' own fade: half way up at 250 ms, full at 500, held 200 ms
    // after the drop and half way down 1 s into the 2 s fall.
    let mut fade = BarsFadeValue::default();
    assert_eq!(fade.step(true, 1000), 0.0);
    assert_eq!(fade.step(true, 1250), 0.5);
    assert_eq!(fade.step(true, 1500), 1.0);
    assert_eq!(fade.step(false, 6000), 1.0);
    assert_eq!(fade.step(false, 6199), 1.0);
    assert_eq!(fade.step(false, 7200), 0.5);
    assert_eq!(fade.step(false, 8200), 0.0);
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

#[test]
fn the_loot_window_is_a_body_of_its_own() {
    // 244 × 440 at x 400; its y is unset and the show centres it at 36.
    assert_eq!(LOOT_SIZE, (244, 440));
    assert_eq!(
        (LOOT_TITLE, LOOT_PLACE),
        ("Body", (400, crate::form::UNPLACED))
    );
    let client = crate::form::ClientRect {
        left: 0,
        top: 0,
        right: 1600,
        bottom: 900,
    };
    assert_eq!(
        crate::form::place_unset(
            LOOT_PLACE.0,
            LOOT_PLACE.1,
            LOOT_SIZE.0,
            LOOT_SIZE.1,
            0x3000,
            client
        ),
        (400, 36)
    );
    // An existing window on the same body is activated; another body opens
    // a second window; a body whose flags' second byte is 3 is refused.
    assert_eq!(loot_screen(0, false), ScreenAction::Opened);
    assert_eq!(loot_screen(0, true), ScreenAction::Focused);
    assert_eq!(loot_screen(0x300, true), ScreenAction::Refused);
    assert_eq!((LOOT_PACK_HEIGHT, LOOT_PANEL_GAP), (180, 4));
}

#[test]
fn the_preview_model_value_and_draw_words() {
    let bound = preview_plan(false);
    assert_eq!(
        bound,
        PreviewPlan {
            model: 0.15,
            scene_word: 0,
            draw_word: 0x42,
            appearance_words: 6,
            build_delay: 100,
        }
    );
    assert_eq!(preview_plan(true).model, 0.05);
    assert_eq!(preview_plan(true).scene_word, 0x1000);
}

#[test]
fn the_status_hide_clock_and_draw_values() {
    assert_eq!(status_hide_until(1000, 500), 1500);
    assert_eq!(status_hide_until(0xffff_fff0, 0x20), 0x10);
    assert_eq!(STATUS_GLOW_RADIUS, 8.0);
    assert_eq!(STATUS_GLOW_SHARE, 0.6);
    assert_eq!(STATUS_ICON_TINT, [140, 160, 255]);
    // The pulse at the glow: a start 1500 ms back is halfway.
    let (pulse, glow) = status_pulse(1500, 0);
    assert!((pulse - 0.5).abs() < 1e-6);
    assert!((glow - 0.3).abs() < 1e-6);
}

#[test]
fn a_selector_technique_byte_nine_marks_the_forget_row() {
    let requirements = [[0; 2]; 8];
    let (rows, forget) = selector_rows(SelectorInput {
        locked: false,
        skill: Skill {
            points: 5000,
            techniques: [9, 8, 0, 0, 0, 0],
        },
        requirements,
        count: 0,
    });
    // The row byte − 1 is written unchecked: 9 reaches the ninth (forget)
    // row and 8 the last technique row.
    assert!(!forget);
    assert_eq!(rows[8], 0x10);
    assert_eq!(rows[7], 0x10);
    assert_eq!(&rows[..7], &[0; 7]);
    // A forget count clears the ninth row again afterwards.
    let (rows, forget) = selector_rows(SelectorInput {
        locked: false,
        skill: Skill {
            points: 5000,
            techniques: [9, 0, 0, 0, 0, 0],
        },
        requirements,
        count: 1,
    });
    assert!(forget);
    assert_eq!(rows[8], 0);
}

#[test]
fn the_power_slot_draw() {
    let slot = PowerSlot {
        icon: 5,
        kind: 0,
        state: 0,
        ..PowerSlot::default()
    };
    let draw = |slot: PowerSlot| slot.draw(10, 20, 40, 40, 2);
    let plain = draw(slot);
    // The frame is inset by the border; a 32-pixel icon is centred four in.
    assert_eq!(plain.frame, (12, 22, 36, 36));
    assert_eq!(plain.icon, (14, 24));
    assert_eq!(plain.tint, SlotTint::Plain);
    assert!(!plain.empty && !plain.extra_frame && !plain.full_scale && !plain.kind_icon);
    // The marked state draws its frame again and takes the marked ink; the
    // selected bit wins over it; kind 2 shares the selected (bright) ink.
    let marked = draw(PowerSlot {
        state: 0x40,
        ..slot
    });
    assert!(marked.extra_frame);
    assert_eq!(marked.tint, SlotTint::Marked);
    assert_eq!(
        draw(PowerSlot {
            state: 0x50,
            ..slot
        })
        .tint,
        SlotTint::Bright
    );
    assert_eq!(draw(PowerSlot { kind: 2, ..slot }).tint, SlotTint::Bright);
    // Under the pointer the icon is drawn at full scale with no ink.
    let hover = draw(PowerSlot {
        state: 0x1_0040,
        ..slot
    });
    assert_eq!(hover.tint, SlotTint::Hover);
    assert!(hover.full_scale && hover.extra_frame);
    // Only the 0x10000 nibble value is hover.
    assert_eq!(
        draw(PowerSlot {
            state: 0x3_0000,
            ..slot
        })
        .tint,
        SlotTint::Plain
    );
    // An empty slot draws its shadow; its kind icon shows for kind 4, or
    // for kind 3 only under the pointer, never for kind 2 or below.
    let empty = PowerSlot { icon: 0, ..slot };
    let e = draw(empty);
    assert!(e.empty && !e.kind_icon && !e.extra_frame);
    assert!(draw(PowerSlot { kind: 4, ..empty }).kind_icon);
    assert!(!draw(PowerSlot { kind: 3, ..empty }).kind_icon);
    let hovered_three = draw(PowerSlot {
        kind: 3,
        state: 0x1_0000,
        ..empty
    });
    assert!(hovered_three.kind_icon && hovered_three.full_scale);
    assert_eq!(hovered_three.tint, SlotTint::Plain);
    assert!(
        !draw(PowerSlot {
            kind: 2,
            state: 0x1_0000,
            ..empty
        })
        .kind_icon
    );
}

#[test]
fn the_preview_build_destroy_and_bars() {
    // The build always answers done; an inventory showing the view refits
    // its camera to ten times the character's height.
    assert_eq!(
        preview_built(true, Some(1.7)),
        PreviewBuild {
            done: true,
            camera: Some(17.0)
        }
    );
    assert_eq!(preview_built(true, None).camera, None);
    assert_eq!(
        preview_built(false, Some(1.7)),
        PreviewBuild {
            done: true,
            camera: None
        }
    );
    assert!(preview_destroyed(true));
    assert!(!preview_destroyed(false));
    // At a 1600 × 1014 view and unit 5: the health bar starts at
    // 800 − 250 = 550, 1014 − 25 = 989, 240 wide; its frame runs from 548
    // to 792 and down to 989 + 18; the focus bar is 260 further right.
    let [health, focus] = preview_bar_rects(1600, 1014, 5);
    assert_eq!(
        health,
        PreviewBar {
            x: 550,
            y: 989,
            width: 240,
            frame: (548, 989, 792, 1007)
        }
    );
    assert_eq!(
        focus,
        PreviewBar {
            x: 810,
            y: 989,
            width: 240,
            frame: (808, 989, 1052, 1007)
        }
    );
    // An odd width halves down.
    assert_eq!(preview_bar_rects(1601, 1014, 5)[0].x, 550);
}

#[test]
fn the_hud_reset_takes_a_hold_only_from_a_fallen_curtain() {
    let mut curtain = LevelCurtain { state: 2, start: 0 };
    let mut holds = 3;
    assert_eq!(curtain.reset_with_hold(&mut holds, true), (true, true));
    assert_eq!(holds, 2);
    assert_eq!(curtain.state, 0);
    curtain.state = 1;
    assert_eq!(curtain.reset_with_hold(&mut holds, false), (false, false));
    assert_eq!(holds, 2);
}

#[test]
fn a_right_release_closes_the_inventory_and_loot() {
    assert!(right_release_closes(GameScreen::Inventory));
    assert!(right_release_closes(GameScreen::Loot));
    assert!(!right_release_closes(GameScreen::Journal));
    assert!(!right_release_closes(GameScreen::TechniqueSelector));
    // The container popup closes on a right click (`popups::container_right_click`).
    assert!(right_release_closes(GameScreen::Container));
    assert_eq!(
        GameScreen::Container.tag(),
        Some(crate::popups::CONTAINER_TAG)
    );
    assert_eq!(GameScreen::Container.size(), None);
}

#[test]
fn the_preview_bars_are_painted_at_the_dolls_bottom() {
    // 800 × 600 at unit 4: the health bar starts at 400 − 200 = 200, 600 −
    // 20 = 580, 192 wide; the focus bar is 208 further right.
    let layout = preview_bar_layout(800, 600, 4);
    assert_eq!(
        (
            layout.health_x,
            layout.focus_x,
            layout.y,
            layout.width,
            layout.height
        ),
        (200, 408, 580, 192, 12)
    );
    let texture = crate::hud::FrameTexture::flat();
    // The two shares are the bar's yellow and its healable total: yellow to
    // 200 + 96, brown to 200 + 144, lasting damage red beyond.
    let canvas = paint_preview_bars((0.5, 0.75), 1.0, 4, (800, 600), &texture);
    let row = 590;
    let yellow = canvas.get(250, row).unwrap();
    let brown = canvas.get(320, row).unwrap();
    let red = canvas.get(370, row).unwrap();
    assert!(
        yellow[3] > 0.9 && brown[3] > 0.7 && red[3] > 0.8,
        "{yellow:?} {brown:?} {red:?}"
    );
    // Yellow is the warm one, red the darkest.
    assert!(yellow[1] > yellow[2], "{yellow:?}");
    assert!(red[0] > red[1], "{red:?}");
    // The focus bar is drawn full.
    let focus = canvas.get(450, row).unwrap();
    assert!(focus[3] > 0.9 && focus[2] > focus[0], "{focus:?}");
    // An empty record leaves the health bar all lasting damage (red).
    let canvas = paint_preview_bars((0.0, 0.0), 1.0, 4, (800, 600), &texture);
    let empty = canvas.get(250, row).unwrap();
    assert!(empty[3] > 0.8, "{empty:?}");
    assert!(empty[0] > empty[1], "{empty:?}");
    // A zero fade paints nothing at all.
    let canvas = paint_preview_bars((1.0, 1.0), 0.0, 4, (800, 600), &texture);
    assert!(canvas.pixels.iter().all(|p| p[3] == 0.0));
}

#[test]
fn the_status_clock_is_stamped_by_the_alert_and_cleared_by_the_hud_switch() {
    let mut clock = StatusClock::default();
    // The marker alone pulses from the unstamped clock (zero): at 1500 ms
    // t = |1000 − 1500| / 1000 = 0.5.
    let draw = clock.frame(true, true, true, false, 1500);
    assert_eq!(draw.marker, Some(0.5));
    assert_eq!(draw.ready, None);
    assert!(!clock.started);
    // The alert's first frame stamps the clock; the marker drawn first in
    // that frame still reads the old stamp (3000 ms from zero is the
    // pulse's low) while the alert starts at its peak.
    let draw = clock.frame(true, true, true, true, 3000);
    assert_eq!(draw.marker, Some(0.0));
    assert_eq!(draw.ready, Some(1.0));
    assert_eq!((clock.start, clock.started), (3000, true));
    // A later frame keeps the stamp: 1000 ms in, the pulse is at its low.
    assert_eq!(clock.frame(true, true, false, true, 4000).ready, Some(0.0));
    assert_eq!(clock.start, 3000);
    // No player: nothing draws and the stamp stays.
    assert_eq!(
        clock.frame(true, false, true, true, 9000),
        StatusDraw::default()
    );
    assert!(clock.started);
    // The HUD switched off clears the stamp; the next alert stamps again.
    assert_eq!(
        clock.frame(false, true, true, true, 9000),
        StatusDraw::default()
    );
    assert!(!clock.started);
    clock.frame(true, true, false, true, 9500);
    assert_eq!(clock.start, 9500);
    // The icon's alpha byte rounds half to even.
    assert_eq!(status_icon_alpha(1.0), 255);
    assert_eq!(status_icon_alpha(0.5), 128); // 127.5 → 128
    assert_eq!(status_icon_alpha(0.0), 0);
    assert_eq!(status_icon_alpha(f32::NAN), 0);
}

#[test]
fn the_bars_show_on_a_health_change_a_focus_drop_or_a_hold() {
    let base = BarsShow {
        last_health: 0.5,
        health: 0.5,
        last_focus: 0.4,
        focus: 0.4,
        power_bar: false,
        combat: false,
        now: 1000,
        hold_until: 0,
    };
    assert!(!bars_shown(base));
    // A changed health shows, both ways.
    assert!(bars_shown(BarsShow {
        health: 0.6,
        ..base
    }));
    assert!(bars_shown(BarsShow {
        health: 0.4,
        ..base
    }));
    // A NaN on either side also shows (the compare is unordered).
    assert!(bars_shown(BarsShow {
        health: f32::NAN,
        ..base
    }));
    assert!(bars_shown(BarsShow {
        last_health: f32::NAN,
        ..base
    }));
    // The focus shows only when it drops, not when it rises, and a NaN
    // focus never shows on its own.
    assert!(bars_shown(BarsShow { focus: 0.3, ..base }));
    assert!(!bars_shown(BarsShow { focus: 0.5, ..base }));
    assert!(!bars_shown(BarsShow {
        focus: f32::NAN,
        ..base
    }));
    // The power bar, combat mode and a hold clock ahead of now (unsigned:
    // a clock past 2^31 is still ahead).
    assert!(bars_shown(BarsShow {
        power_bar: true,
        ..base
    }));
    assert!(bars_shown(BarsShow {
        combat: true,
        ..base
    }));
    assert!(bars_shown(BarsShow {
        hold_until: 0x8000_0000,
        ..base
    }));
    assert!(bars_shown(BarsShow {
        hold_until: 1001,
        ..base
    }));
    assert!(!bars_shown(BarsShow {
        hold_until: 1000,
        ..base
    }));
}

#[test]
fn the_gauge_needs_a_shield_and_combat() {
    assert!(gauge_shown(true, true));
    assert!(!gauge_shown(true, false));
    assert!(!gauge_shown(false, true));
    assert!(!gauge_shown(false, false));
}

#[test]
fn the_inventory_page_button_states() {
    assert_eq!(inventory_tab_states(0), [0x60, 0]);
    assert_eq!(inventory_tab_states(0x0100_0000), [0, 0x60]);
    assert_eq!(inventory_tab_states(0xfeff_ffff), [0x60, 0]);
}

#[test]
fn the_inventory_fill_writes_the_page_and_place() {
    let fill = inventory_fill(InventoryFillInput {
        owner_is_player: true,
        remembered: Some((12, 34)),
        screen_width: 1600,
        width: 244,
        unit: 4,
        current: (8, 8),
        flags: 0,
        view_mounted: true,
    });
    assert_eq!(fill.view_model, Some(0.05));
    assert_eq!(fill.scene_word, Some(0x1000));
    assert_eq!(fill.tab_states, [0x60, 0]);
    assert_eq!(fill.place, (12, 34));
    // Without a mounted view no model or scene word is written; another
    // character's window goes to 1600 − 244 − 16 = 1340.
    let fill = inventory_fill(InventoryFillInput {
        owner_is_player: false,
        remembered: None,
        screen_width: 1600,
        width: 244,
        unit: 4,
        current: (8, 8),
        flags: 0x0100_0000,
        view_mounted: false,
    });
    assert_eq!(fill.view_model, None);
    assert_eq!(fill.scene_word, None);
    assert_eq!(fill.tab_states, [0, 0x60]);
    assert_eq!(fill.place, (1340, 8));
}

#[test]
fn the_counter_answers_its_messages() {
    let mut counter = Counter {
        value: 0.0,
        segments: 3,
    };
    assert_eq!(counter.message(1, 0.0).draw, ControlDraw::Segments);
    assert_eq!(counter.message(2, 0.0).draw, ControlDraw::Skin);
    assert_eq!(counter.message(3, 0.0).result, 0);
    let answer = counter.message(0x200, 0.5);
    assert_eq!(answer.result, 3);
    assert_eq!(answer.caption.as_deref(), Some("50.0%"));
    assert!(answer.callback);
    assert_eq!(counter.value, 0.5);
    // A negative value is clamped; −0 and a NaN stay, and the drawing's
    // share test treats them as the original does.
    counter.message(0x200, -1.0);
    assert_eq!(counter.value, 0.0);
    counter.message(0x200, -0.0);
    assert!(counter.value.is_sign_negative() && counter.value == 0.0);
    counter.message(0x200, f32::NAN);
    assert!(counter.value.is_nan());
}

#[test]
fn the_bar_control_draws_two_pixels_in() {
    let bar = BarControl {
        first: 0.75,
        second: 0.5,
    };
    assert_eq!(bar.message(1, 10, 20, 30), BarDraw::Frame);
    assert_eq!(bar.message(3, 10, 20, 30), BarDraw::None);
    assert_eq!(
        bar.message(2, 10, 20, 30),
        BarDraw::Bar {
            x: 12,
            y: 21,
            width: 26,
            first: 0.75,
            second: 0.5
        }
    );
}

#[test]
fn the_journal_list_state() {
    let mut journal = Journal::default();
    journal.add_entry("first");
    journal.entries[1].text = "second".to_string();
    journal.current = 1;
    let list = journal.list_state();
    assert_eq!(list.title, "New Entry");
    assert_eq!(list.text, "second");
    assert_eq!(list.selected, 2);
    assert_eq!(list.count, 2);
    assert_eq!(list.titles, vec!["New Entry", "New Entry"]);
}

#[test]
fn the_selector_opens_at_the_records_corner() {
    let skill = Skill {
        points: 10_000,
        techniques: [1, 2, 0, 0, 0, 0],
    };
    let open = selector_open(SelectorOpenInput {
        x: 100,
        y: 200,
        width: 40,
        height: 40,
        rows: SelectorInput {
            locked: false,
            skill,
            requirements: [[0; 2]; 8],
            count: 1,
        },
    });
    assert_eq!(open.place.desired_x, 140);
    assert_eq!(open.place.fallback_x, 100);
    assert_eq!(open.place.desired_y, 201);
    assert_eq!(open.place.fallback_bottom, 239);
    assert_eq!(open.rows[0], 0x10);
    assert_eq!(open.rows[1], 0x10);
    assert_eq!(open.rows[8], 0);
    assert!(open.forget);
    assert_eq!(open.record_state, 0x60);
}

#[test]
fn the_deferred_fill_retries_until_the_node_exists() {
    assert_eq!(
        async_inventory_fill(true),
        AsyncFill {
            fill: true,
            retry: false
        }
    );
    assert_eq!(
        async_inventory_fill(false),
        AsyncFill {
            fill: false,
            retry: true
        }
    );
}
