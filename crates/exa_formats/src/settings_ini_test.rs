use super::*;

fn values(text: &[u8]) -> Vec<(&'static str, Value)> {
    read_assignments(text, &REGISTRY)
        .into_iter()
        .map(|a| (REGISTRY[a.index].name, a.value))
        .collect()
}

#[test]
fn lines_split_at_the_first_equals_or_space() {
    let text = b"\r\nGUISize = 2\r\nfullscreen=1\r\nNo such = 3\r\nSupersample 4\r\n\
                 FPSLimit =   60  \r\nnothing here\r\nab\x01TextureDetail=7";
    assert_eq!(
        values(text),
        [
            ("GUISize", Value::Int(2)),
            ("Fullscreen", Value::Bool(true)),
            ("Supersample", Value::Int(4)),
            ("FPSLimit", Value::Int(60)),
            // A control byte inside the name restarts it; the last line runs
            // to the end of the file.
            ("TextureDetail", Value::Int(7)),
        ]
    );
}

#[test]
fn the_value_loses_the_byte_before_its_line_feed() {
    // With CR LF the CR goes; with a bare LF the value's last digit does.
    assert_eq!(values(b"GUISize = 12\r\n"), [("GUISize", Value::Int(12))]);
    assert_eq!(values(b"GUISize = 12\n"), [("GUISize", Value::Int(1))]);
    // A one-byte value before a bare LF is empty and sets nothing.
    assert!(values(b"GUISize = 3\nShadowQuality = 2\r\n").len() == 1);
    assert!(values(b"").is_empty());
    assert!(values(b"GUISize =\r\n").is_empty());
}

#[test]
fn values_convert_by_kind() {
    let text = b"PerfIndex = 1.25\r\nVolume = -0.5\r\nCursorSize = 3.\r\n\
                 CursorInt = ff8000\r\nCursorCom = 11223344\r\nMusic = YES\r\n\
                 ShowFPS = 1x\r\nCamFollow = TRUE\r\nLinearRGB = no\r\nKeyDef = alt+q: a; bc\r\n";
    assert_eq!(
        values(text),
        [
            ("PerfIndex", Value::Float(1.25)),
            ("Volume", Value::Float(-0.5)),
            ("CursorSize", Value::Float(3.0)),
            ("CursorInt", Value::Colour(0xff80_0000_u32 as i32)),
            ("CursorCom", Value::Colour(0x1122_3344)),
            ("Music", Value::Bool(true)),
            ("ShowFPS", Value::Bool(true)),
            ("CamFollow", Value::Bool(true)),
            ("LinearRGB", Value::Bool(false)),
            ("KeyDef", Value::Text(b"alt+q: a; bc".to_vec())),
        ]
    );
    assert_eq!(parse_int(b"-42x"), -42);
    assert_eq!(parse_int(b"-"), 0);
    assert_eq!(parse_int(b"x1"), 0);
    // 2^32 + 5 cut to 32 bits.
    assert_eq!(parse_int(b"4294967301"), 5);
    assert_eq!(parse_float(b"0.125"), 0.125);
    assert_eq!(parse_float(b"7.5x9"), 7.5);
    assert_eq!(parse_hex(b"7fG"), 0x7f);
}

#[test]
fn the_graphics_rules_clamp_and_derive() {
    let read = |text: &[u8], height| Settings::read(text, height).0;
    let s = read(
        b"ShadingQuality = 1\r\nShadowDetail = 0\r\nTextureQuality = 1\r\n\
          TextureDetail = -3\r\nSupersample = 9\r\nFPSLimit = 60\r\nVerticalSync = 0\r\n",
        1080,
    );
    assert_eq!(s.shading_quality, 3);
    assert_eq!(s.shading_flags, 0x273);
    assert!(!s.shading_extra);
    assert_eq!(
        (s.shadow_detail, s.texture_quality, s.texture_detail),
        (3, 2, 2)
    );
    assert_eq!(s.supersample, 4);
    assert_eq!(s.fps_cap, 62);
    assert!(!s.vertical_sync);
    let s = read(b"ShadingQuality = 6\r\n", 1080);
    assert_eq!(
        (s.shading_quality, s.shading_flags, s.shading_extra),
        (6, 0x2f3, true)
    );
    // The defaults before any line.
    let d = Settings::default();
    assert_eq!(
        (
            d.shading_quality,
            d.shadow_quality,
            d.shadow_detail,
            d.texture_quality,
            d.texture_detail
        ),
        (4, 4, 4, 4, 3)
    );
    assert_eq!(
        (d.window_width, d.window_height, d.fps_cap),
        (1600, 1080, -1)
    );
    assert!(d.vertical_sync && !d.fullscreen);
}

#[test]
fn a_positive_gui_size_needs_a_display_800_tall() {
    let read = |text: &[u8], height| Settings::read(text, height).0;
    let s = read(b"GUISize = 2\r\n", 1080);
    assert_eq!((s.gui_size, s.compact_display), (0, false));
    let s = read(b"GUISize = 2\r\n", 800);
    assert_eq!((s.gui_size, s.compact_display), (1, true));
    let s = read(b"GUISize = -1\r\n", 800);
    assert_eq!((s.gui_size, s.compact_display), (-1, false));
}

#[test]
fn the_frame_cap_goes_through_whole_milliseconds() {
    assert_eq!(fps_cap(9), 0);
    assert_eq!(fps_cap(10), 10);
    assert_eq!(fps_cap(30), 30);
    assert_eq!(fps_cap(60), 62);
    assert_eq!(fps_cap(144), 166);
    assert_eq!(fps_cap(1000), 1000);
    assert_eq!(fps_cap(1001), 0);
}

#[test]
fn the_settings_screen_replaces_the_line() {
    let mut f = b"\r\nA = 1\r\nGUISize = 2\r\nB = 3".to_vec();
    put_setting(&mut f, b"GUISize", b"5");
    assert_eq!(f, b"\r\nA = 1\r\nGUISize = 5\r\nB = 3");
    // At the very start, and on a last line without a CR.
    let mut f = b"GUISize = 2".to_vec();
    put_setting(&mut f, b"GUISize", b"10");
    assert_eq!(f, b"GUISize = 10");
    let mut f = Vec::new();
    put_setting(&mut f, b"Fullscreen", b"1");
    assert_eq!(f, b"\r\nFullscreen = 1");
}

#[test]
fn the_game_save_starts_one_byte_early() {
    let mut f = b"\r\nA = 1\r\nGUISize = 2\r\nB = 3".to_vec();
    save_one_setting(&mut f, b"GUISize", b"5");
    // The line feed before the name goes, so it joins the line before.
    assert_eq!(f, b"\r\nA = 1\rGUISize = 5\r\nB = 3");
    let (s, _) = Settings::read(&f, 1080);
    assert_eq!(s.gui_size, 0);
    // A name opening the file counts as missing: the line is appended.
    let mut f = b"GUISize = 2".to_vec();
    save_one_setting(&mut f, b"GUISize", b"5");
    assert_eq!(f, b"GUISize = 2\r\nGUISize = 5");
    // The first run's save into a new, empty file.
    let mut f = Vec::new();
    save_one_setting(&mut f, b"FirstRun", format_bool(false).as_bytes());
    assert_eq!(f, b"\r\nFirstRun = 0");
    assert_eq!(values(&f), [("FirstRun", Value::Bool(false))]);
}

#[test]
fn the_text_search_is_zero_based() {
    assert_eq!(find_text(b"xy", b"abcxy", 0), 3);
    assert_eq!(find_text(b"zz", b"abcxy", 0), -1);
    assert_eq!(find_text(b"a", b"ab", 1), 0);
    assert_eq!(find_text(b"a", b"xab", 2), 1);
    assert_eq!(find_text(b"a", b"xab", 3), -1);
    assert_eq!(find_text(b"ab", b"ab", 2), -1);
    assert_eq!(find_text(b"", b"ab", 0), -1);
}

#[test]
fn numbers_are_written_as_the_original_writes_them() {
    assert_eq!(format_int(0), "0");
    assert_eq!(format_int(150), "150");
    assert_eq!(format_int(-42), "-42");
    assert_eq!(format_int(i32::MIN), "-");
    assert_eq!(format_float(1.25, 2, true), "1.25");
    assert_eq!(format_float(0.5, 2, true), "0.50");
    assert_eq!(format_float(0.5, 2, false), "0.5");
    assert_eq!(format_float(2.0, 2, false), "2.0");
    assert_eq!(format_float(0.999, 2, true), "1.00");
    assert_eq!(format_float(-1.999, 2, true), "-2.00");
    assert_eq!(format_float(-0.25, 2, true), "-0.25");
    assert_eq!(format_float(2.5, 0, true), "2");
    assert_eq!(format_float(3.5, 0, true), "4");
}

#[test]
fn key_defs_bind_a_modifier_a_key_and_commands() {
    let mut keys = Vec::new();
    parse_key_def(b"ctrl+k: spawn sword; heal  ;x", &mut keys);
    parse_key_def(b"shift + A : go", &mut keys);
    parse_key_def(b"meta+x: a", &mut keys);
    parse_key_def(b"ctrl+x:", &mut keys);
    parse_key_def(b"alt+x", &mut keys);
    assert_eq!(
        keys,
        [
            Hotkey {
                modifier: CTRL,
                key: b'K',
                // The trailing single byte is dropped.
                commands: vec![b"spawn sword".to_vec(), b"heal".to_vec()],
                bound: true,
            },
            Hotkey {
                modifier: SHIFT,
                key: b'A',
                commands: vec![b"go".to_vec()],
                bound: true,
            },
        ]
    );
    for _ in 0..5 {
        parse_key_def(b"alt+z: ab", &mut keys);
    }
    assert_eq!(keys.len(), 7);
    assert!(keys[5].bound && !keys[6].bound);
    assert_eq!(keys[6].modifier, ALT);
}

#[test]
fn the_user_folder_and_its_files() {
    let game = Path::new("/games/Exanima");
    let app = Path::new("/home/u/AppData");
    assert_eq!(
        user_folder(game, true, app, "Exanima"),
        game.join("Userdata")
    );
    let folder = user_folder(game, false, app, "Exanima");
    assert_eq!(folder, app.join("Exanima"));
    assert_eq!(
        settings_file(&folder, "Exanima"),
        app.join("Exanima/Exanima.ini")
    );
    assert_eq!(
        store_file(&folder, "Exanima"),
        app.join("Exanima/Exanima.set")
    );
    assert!(sets(
        &read_assignments(b"Fullscreen = 1\r\n", &REGISTRY),
        "Fullscreen"
    ));
    assert!(!sets(
        &read_assignments(b"Fullscreen = 1\r\n", &REGISTRY),
        "GUISize"
    ));
}
