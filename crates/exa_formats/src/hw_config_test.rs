use super::*;

#[test]
fn the_defaults_are_the_original_variables() {
    let o = Options::default();
    assert_eq!(o.volume, 1.0);
    assert!(o.music);
    assert_eq!(o.supersample, 0);
    assert_eq!(o.shading_quality, 4);
    assert_eq!(o.texture_detail, 3);
    assert_eq!(o.texture_quality, 4);
    assert_eq!(o.shadow_detail, 4);
    assert_eq!(o.shadow_quality, 4);
    assert_eq!(o.multisample, 0);
    assert!(o.vertical_sync);
    assert!(!o.show_fps);
    assert!(o.window_mode);
    assert!(o.show_tips);
    assert!(o.checkpoints);
    assert_eq!(o.aim_speed, 0.29);
    assert!(!o.controller);
    assert_eq!(o.perf_index, 0.0);
}

#[test]
fn the_label_scales_take_exactly_their_values() {
    assert_eq!(Scale::Ultra.text(-5), Some(" Off"));
    assert_eq!(Scale::Ultra.text(2), Some(" Low"));
    assert_eq!(Scale::Ultra.text(6), Some(" Ultra"));
    assert_eq!(Scale::Ultra.text(7), None);

    assert_eq!(Scale::Coarse.text(2), Some(" Coarse"));
    assert_eq!(Scale::Coarse.text(3), Some(" Low"));
    assert_eq!(Scale::Coarse.text(6), Some(" Very high"));
    assert_eq!(Scale::Coarse.text(7), None);

    assert_eq!(Scale::CoarseOff.text(0), Some(" Off"));
    assert_eq!(Scale::CoarseOff.text(1), Some(" Coarse"));
    assert_eq!(Scale::CoarseOff.text(5), Some(" Very high"));
    assert_eq!(Scale::CoarseOff.text(-1), None);
    assert_eq!(Scale::CoarseOff.text(6), None);

    assert_eq!(Scale::Supersample.text(0), Some(" Off"));
    assert_eq!(Scale::Supersample.text(1), Some(" Low"));
    assert_eq!(Scale::Supersample.text(2), None);
    assert_eq!(Scale::Supersample.text(3), Some(" Medium"));
    assert_eq!(Scale::Supersample.text(4), Some(" High"));
    assert_eq!(Scale::Supersample.text(5), Some(" Very high"));
    assert_eq!(Scale::Supersample.text(6), None);

    assert_eq!(Scale::LowHigh.text(0), Some(" Low"));
    assert_eq!(Scale::LowHigh.text(1), Some(" High"));
    assert_eq!(Scale::LowHigh.text(2), None);

    assert_eq!(Scale::Samples.text(0), Some(" Off"));
    assert_eq!(Scale::Samples.text(1), Some(" 2x"));
    assert_eq!(Scale::Samples.text(2), Some(" 4x"));
    assert_eq!(Scale::Samples.text(3), Some(" 8x"));
    assert_eq!(Scale::Samples.text(4), None);
}

#[test]
fn the_slider_fractions_round_trip() {
    assert_eq!(supersample_from_fraction(0.0), 0);
    assert_eq!(supersample_from_fraction(1.0 / 3.0), 1);
    assert_eq!(supersample_from_fraction(2.0 / 3.0), 3);
    assert_eq!(supersample_from_fraction(1.0), 4);
    for level in [0, 1, 3, 4] {
        let o = Options {
            supersample: level,
            ..Options::default()
        };
        assert_eq!(supersample_from_fraction(o.supersample_fraction()), level);
    }

    assert_eq!(shading_from_fraction(0.0), 3);
    assert_eq!(shading_from_fraction(1.0 / 3.0), 4);
    assert_eq!(shading_from_fraction(2.0 / 3.0), 5);
    assert_eq!(shading_from_fraction(1.0), 6);
    for value in 3..=6 {
        let o = Options {
            shading_quality: value,
            ..Options::default()
        };
        assert_eq!(shading_from_fraction(o.shading_fraction()), value);
    }

    assert_eq!(texture_detail_from_fraction(0.0), 2);
    // Ties go to even: a slider at one half keeps the even step.
    assert_eq!(texture_detail_from_fraction(0.5), 2);
    assert_eq!(texture_detail_from_fraction(1.0), 3);

    assert_eq!(texture_quality_from_fraction(0.0), 2);
    assert_eq!(texture_quality_from_fraction(1.0 / 3.0), 3);
    assert_eq!(texture_quality_from_fraction(2.0 / 3.0), 4);
    assert_eq!(texture_quality_from_fraction(1.0), 5);
    for value in 2..=5 {
        let o = Options {
            texture_quality: value,
            ..Options::default()
        };
        assert_eq!(
            texture_quality_from_fraction(o.texture_quality_fraction()),
            value
        );
    }

    assert_eq!(shadow_detail_from_fraction(0.0), 4);
    assert_eq!(shadow_detail_from_fraction(0.5), 4);
    assert_eq!(shadow_detail_from_fraction(1.0), 5);

    assert_eq!(shadow_quality_from_fraction(0.0), 1);
    assert_eq!(shadow_quality_from_fraction(1.0 / 3.0), 2);
    assert_eq!(shadow_quality_from_fraction(2.0 / 3.0), 3);
    assert_eq!(shadow_quality_from_fraction(1.0), 4);
    for value in 1..=4 {
        let o = Options {
            shadow_quality: value,
            ..Options::default()
        };
        assert_eq!(
            shadow_quality_from_fraction(o.shadow_quality_fraction()),
            value
        );
    }

    assert_eq!(multisample_from_fraction(0.0), 0);
    assert_eq!(multisample_from_fraction(1.0 / 3.0), 2);
    // 3 · 0.5 = 1.5 rounds to even, which is 2, giving 4 samples.
    assert_eq!(multisample_from_fraction(0.5), 4);
    assert_eq!(multisample_from_fraction(2.0 / 3.0), 4);
    assert_eq!(multisample_from_fraction(1.0), 8);
    for samples in [0, 2, 4, 8] {
        let o = Options {
            multisample: samples,
            ..Options::default()
        };
        assert_eq!(multisample_from_fraction(o.multisample_fraction()), samples);
    }

    assert_eq!(volume_from_fraction(0.0), 0.0);
    assert_eq!(volume_from_fraction(0.5), 0.75);
    assert!((volume_from_fraction(Options::default().volume_fraction()) - 1.0).abs() < 1e-6);
}

#[test]
fn the_floor_logarithm_counts_halvings() {
    assert_eq!(floor_log2(0), 0);
    assert_eq!(floor_log2(-4), 0);
    assert_eq!(floor_log2(1), 0);
    assert_eq!(floor_log2(2), 1);
    assert_eq!(floor_log2(3), 1);
    assert_eq!(floor_log2(4), 2);
    assert_eq!(floor_log2(8), 3);
    assert_eq!(floor_log2(9), 3);
}

#[test]
fn the_slider_labels_follow_the_values() {
    let mut o = Options::default();
    assert_eq!(o.volume_fraction(), 2.0 / 3.0);
    assert_eq!(o.supersample_text(), Some(" Off"));
    assert_eq!(o.shading_text(), Some(" Medium"));
    assert_eq!(o.texture_detail_text(), Some(" High"));
    assert_eq!(o.texture_quality_text(), Some(" High"));
    assert_eq!(o.shadow_detail_text(), Some(" Low"));
    assert_eq!(o.shadow_quality_text(), Some(" High"));
    assert_eq!(o.multisample_text(), Some(" Off"));
    assert_eq!(o.aim_speed_text(), 30);

    o.supersample = 3;
    assert_eq!(o.supersample_text(), Some(" Medium"));
    o.shading_quality = 6;
    assert_eq!(o.shading_text(), Some(" Very high"));
    o.texture_detail = 2;
    assert_eq!(o.texture_detail_text(), Some(" Low"));
    o.texture_quality = 2;
    assert_eq!(o.texture_quality_text(), Some(" Low"));
    o.shadow_detail = 5;
    assert_eq!(o.shadow_detail_text(), Some(" High"));
    o.shadow_quality = 1;
    assert_eq!(o.shadow_quality_text(), Some(" Coarse"));
    o.multisample = 4;
    assert_eq!(o.multisample_text(), Some(" 4x"));
    o.aim_speed = 0.0;
    assert_eq!(o.aim_speed_text(), 1);
    o.aim_speed = 1.0;
    assert_eq!(o.aim_speed_text(), 100);
}

#[test]
fn the_auto_quality_tiers() {
    let low = Quality::auto(0.59, 1600, 900);
    assert_eq!(low.supersample, 0);
    assert_eq!(low.shading_quality, 3);
    assert_eq!(low.texture_detail, 2);
    assert_eq!(low.texture_quality, 2);
    assert_eq!(low.shadow_detail, 4);
    assert_eq!(low.shadow_quality, 1);

    let low_high = Quality::auto(0.6, 1600, 900);
    assert_eq!(low_high.shadow_quality, 2);
    assert_eq!(Quality::auto(0.99, 1600, 900), low_high);

    let middle = Quality::auto(1.0, 1600, 900);
    assert_eq!(middle.supersample, 0);
    assert_eq!(middle.shading_quality, 3);
    assert_eq!(middle.texture_detail, 2);
    assert_eq!(middle.texture_quality, 3);
    assert_eq!(middle.shadow_detail, 4);
    assert_eq!(middle.shadow_quality, 2);
    assert_eq!(Quality::auto(1.49, 1600, 900), middle);

    let top = Quality::auto(1.5, 1600, 900);
    assert_eq!(top.supersample, 0);
    assert_eq!(top.shading_quality, 5);
    assert_eq!(top.texture_detail, 3);
    assert_eq!(top.texture_quality, 5);
    assert_eq!(top.shadow_detail, 5);
    assert_eq!(top.shadow_quality, 4);

    // √(2304000 / 1440000) = 1.2649…; the super-sampling threshold 3 is
    // crossed near a performance index of 2.3717.
    assert_eq!(Quality::auto(2.37, 1600, 900).supersample, 0);
    assert_eq!(Quality::auto(2.38, 1600, 900).supersample, 1);
    assert_eq!(Quality::auto(3.95, 1600, 900).supersample, 1);
    assert_eq!(Quality::auto(3.96, 1600, 900).supersample, 3);
    assert_eq!(Quality::auto(7.90, 1600, 900).supersample, 3);
    assert_eq!(Quality::auto(7.91, 1600, 900).supersample, 4);
}

#[test]
fn the_auto_quality_takes_nan_as_not_below() {
    let nan = Quality::auto(f32::NAN, 1600, 900);
    assert_eq!(nan.supersample, 4);
    assert_eq!(nan.shading_quality, 5);
    assert_eq!(nan.shadow_quality, 4);
    // A zero or negative display area gives an infinite or NaN level, which
    // also lands on the top level.
    assert_eq!(Quality::auto(2.0, 0, 0).supersample, 4);
    assert_eq!(Quality::auto(2.0, -1600, 900).supersample, 4);
}

#[test]
fn opening_the_screen_keeps_some_values_and_resets_others() {
    let live = Options {
        volume: 0.5,
        music: false,
        supersample: 3,
        shading_quality: 3,
        texture_detail: 2,
        texture_quality: 5,
        shadow_detail: 5,
        shadow_quality: 2,
        multisample: 8,
        vertical_sync: false,
        show_fps: true,
        window_mode: false,
        show_tips: false,
        checkpoints: false,
        aim_speed: 0.4,
        controller: true,
        perf_index: 1.2,
    };
    let draft = Options::opened_from(&live);
    assert_eq!(draft.volume, 0.5);
    assert!(!draft.music);
    assert_eq!(draft.supersample, 3);
    assert_eq!(draft.shading_quality, 3);
    assert_eq!(draft.texture_detail, 2);
    assert_eq!(draft.texture_quality, 5);
    assert_eq!(draft.shadow_detail, 5);
    assert_eq!(draft.shadow_quality, 2);
    assert_eq!(draft.multisample, 0);
    assert!(draft.vertical_sync);
    assert!(draft.show_fps);
    assert!(!draft.window_mode);
    assert!(draft.show_tips);
    assert!(draft.checkpoints);
    assert_eq!(draft.aim_speed, 0.4);
    assert!(draft.controller);
    assert_eq!(draft.perf_index, 1.2);
}

#[test]
fn the_auto_detection_fills_the_quality_and_resets_the_rest() {
    let live = Options {
        volume: 0.8,
        music: false,
        multisample: 8,
        vertical_sync: false,
        window_mode: false,
        show_fps: true,
        show_tips: false,
        checkpoints: false,
        aim_speed: 0.5,
        controller: true,
        ..Options::default()
    };
    let auto = Options::auto_detected(&live, 4.0, 1600, 900);
    assert_eq!(auto.supersample, 3);
    assert_eq!(auto.shading_quality, 5);
    assert_eq!(auto.texture_detail, 3);
    assert_eq!(auto.texture_quality, 5);
    assert_eq!(auto.shadow_detail, 5);
    assert_eq!(auto.shadow_quality, 4);
    assert_eq!(auto.multisample, 0);
    assert!(auto.vertical_sync);
    assert!(auto.show_tips);
    assert!(auto.checkpoints);
    assert_eq!(auto.volume, 0.8);
    assert!(!auto.music);
    assert!(!auto.window_mode);
    assert!(auto.show_fps);
    assert_eq!(auto.aim_speed, 0.5);
    assert!(auto.controller);
    assert_eq!(auto.perf_index, 4.0);
}

#[test]
fn the_deck_preset() {
    let live = Options::default();
    let deck = Options::deck_preset(&live);
    assert_eq!(deck.supersample, 3);
    assert_eq!(deck.shading_quality, 5);
    assert_eq!(deck.texture_detail, 2);
    assert_eq!(deck.texture_quality, 5);
    assert_eq!(deck.shadow_detail, 5);
    assert_eq!(deck.shadow_quality, 3);
    assert_eq!(deck.multisample, 0);
    assert!(deck.vertical_sync);
    assert_eq!(deck.perf_index, live.perf_index);
}

#[test]
fn apply_writes_its_lines_in_the_original_order() {
    let live = Options {
        perf_index: 1.234,
        volume: 0.5,
        aim_speed: 0.29,
        music: true,
        show_fps: true,
        show_tips: false,
        checkpoints: true,
        ..Options::default()
    };
    let mut screen = Screen::open(&live);
    screen.draft.supersample = 3;
    screen.draft.shading_quality = 5;
    screen.draft.texture_detail = 3;
    screen.draft.texture_quality = 5;
    screen.draft.shadow_detail = 5;
    screen.draft.shadow_quality = 4;
    screen.draft.multisample = 4;
    screen.draft.controller = true;
    screen.draft.window_mode = false;
    screen.draft.vertical_sync = false;

    let lines = screen.settings_lines(&live);
    let expected = [
        ("PerfIndex", "1.23"),
        ("Volume", "50"),
        ("Supersample", "3"),
        ("ShadingQuality", "5"),
        ("TextureDetail", "3"),
        ("TextureQuality", "5"),
        ("ShadowDetail", "5"),
        ("ShadowQuality", "4"),
        ("HWMultisample", "4"),
        ("InputMode", "51"),
        ("AimSpeed", "30"),
        ("Fullscreen", "1"),
        ("Music", "1"),
        ("ShowFPS", "1"),
        ("ShowGameTips", "0"),
        ("Checkpoints", "1"),
        ("VerticalSync", "0"),
        ("ShowFPS", "1"),
    ];
    let got: Vec<(&str, &str)> = lines
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    assert_eq!(got, expected);

    let mut file = Vec::new();
    screen.write_settings(&mut file, &live);
    let text = String::from_utf8(file).unwrap();
    assert!(text.contains("\r\nPerfIndex = 1.23\r\n"));
    // The second ShowFPS write rewrites the line the first made, so the
    // file holds it once.
    assert_eq!(text.matches("ShowFPS").count(), 1);

    // The Music checkbox does not change the live music value: the file
    // keeps what the startup settings gave.
    screen.draft.music = false;
    let lines = screen.settings_lines(&live);
    assert_eq!(lines[12], ("Music", "1".to_string()));
}

#[test]
fn apply_reports_the_quality_changes_alone() {
    let live = Options::default();
    let mut screen = Screen::open(&live);
    assert_eq!(screen.quality_change(), QualityChange::default());
    assert!(!screen.quality_change().rebuilds_shaders());

    screen.draft.volume = 1.4;
    screen.draft.vertical_sync = false;
    screen.draft.window_mode = false;
    assert_eq!(screen.quality_change(), QualityChange::default());

    screen.draft.shadow_detail = 5;
    let change = screen.quality_change();
    assert!(change.shadow_detail);
    assert!(!change.shadow_quality);
    assert!(change.rebuilds_shaders());

    screen.draft.supersample = 1;
    let change = screen.quality_change();
    assert!(change.supersample && change.shadow_detail);
}

#[test]
fn the_screen_cancel_restores_the_live_options() {
    let live = Options {
        volume: 1.2,
        supersample: 1,
        ..Options::default()
    };
    let mut screen = Screen::open(&live);
    screen.draft.supersample = 4;
    screen.draft.volume = 0.1;
    assert_eq!(screen.cancel(), live);
}

#[test]
fn the_benchmark_counts_half_second_runs_and_keeps_the_best() {
    let mut benchmark = Benchmark::new(1000);
    assert_eq!(benchmark.half(), 500);
    assert!(!benchmark.running());

    // Warm-up: five frames of 100 ticks.
    for now in [100, 200, 300, 400, 500] {
        assert_eq!(benchmark.frame(now), None);
    }
    assert!(benchmark.running());

    // Run 1: five frames.
    for now in [600, 700, 800, 900, 1000] {
        assert_eq!(benchmark.frame(now), None);
    }

    // Run 2: ten frames of 50 ticks.
    for now in [1050, 1100, 1150, 1200, 1250, 1300, 1350, 1400, 1450, 1500] {
        assert_eq!(benchmark.frame(now), None);
    }

    // Run 3: five frames, then the index from the best rate (10).
    let mut result = None;
    for now in [1600, 1700, 1800, 1900, 2000] {
        result = benchmark.frame(now);
        if now < 2000 {
            assert_eq!(result, None);
        }
    }
    let perf = result.expect("the third run ends the benchmark");
    assert!((perf - 10.0 / 60.0).abs() < 1e-7);
}

#[test]
fn the_benchmark_rounds_its_rate_to_even_and_survives_a_zero_frequency() {
    assert_eq!(perf_index(60), 1.0);
    assert_eq!(perf_index(0), 0.0);
    assert_eq!(perf_index(15), 0.25);

    // Half a tick per phase: every frame closes one.
    let mut benchmark = Benchmark::new(0);
    assert_eq!(benchmark.half(), 1);
    assert_eq!(benchmark.frame(0), None); // still inside the warm-up tick
    assert_eq!(benchmark.frame(1), None); // warm-up
    assert_eq!(benchmark.frame(2), None); // run 1
    assert_eq!(benchmark.frame(3), None); // run 2
    // Run 3: one frame in one tick, so a rate of 1; the best stays 1.
    let perf = benchmark.frame(4).unwrap();
    assert_eq!(perf, 1.0 / 60.0);

    // A counter that goes backwards restarts the phase instead of panicking.
    let mut benchmark = Benchmark::new(1000);
    assert_eq!(benchmark.frame(10), None);
    assert_eq!(benchmark.frame(5), None);
    assert_eq!(benchmark.frame(600), None);
}

#[test]
fn the_key_names_follow_the_mouse_and_extended_shapes() {
    assert_eq!(key_name(0x8004), "Mouse 3");
    assert_eq!(key_name(0x8005), "Mouse 4");
    assert_eq!(key_name(0x8006), "Mouse 5");
    assert_eq!(key_name(0x8007), "");
    assert_eq!(key_name(0x4004), "<4>");
    // The mouse bit wins over the extended bit.
    assert_eq!(key_name(0xC004), "Mouse 3");
    assert_eq!(key_name(0x57), "W");
    assert_eq!(key_name(0x10), "Shift");
    assert_eq!(key_name(0xA0), "Left Shift");
    assert_eq!(key_name(0x0D), "Enter");
    assert_eq!(key_name(0x20), "Space");
    assert_eq!(key_name(0x99), "<153>");
}

#[test]
fn a_key_name_is_title_cased() {
    assert_eq!(title_case("LEFT SHIFT"), "Left Shift");
    assert_eq!(title_case("PAGE_UP"), "Page Up");
    assert_eq!(title_case("F1"), "F1");
    assert_eq!(title_case("A1B2"), "A1B2");
    assert_eq!(title_case("f1"), "F1");
    assert_eq!(title_case("MOUSE 3"), "Mouse 3");
}

#[test]
fn the_device_and_memory_lines() {
    assert_eq!(device_text(Some("GeForce"), false), "GeForce");
    assert_eq!(device_text(None, false), "");
    assert_eq!(device_text(None, true), "Valve Steam Deck");
    assert_eq!(memory_text(0), "Unknown");
    assert_eq!(memory_text(4096), "4096 MB");
}

#[test]
fn the_aim_speed_file_step_is_one_based() {
    assert_eq!(aim_speed_from_step(0), 0.0);
    assert_eq!(aim_speed_from_step(1), 0.0);
    assert!((aim_speed_from_step(30) - 29.0 / 99.0).abs() < 1e-7);
    assert_eq!(aim_speed_from_step(100), 1.0);
    assert_eq!(aim_speed_from_step(1000), 1.0);
    assert_eq!(aim_speed_from_step(-5), 0.0);
}

#[test]
fn the_hardware_settings_are_read_from_the_file() {
    let bytes = b"AimSpeed = 30\r\nPerfIndex = 1.5\r\nShowFPS = 1\r\nMusic = 0\r\n\
                  ScreenWidth = 1280\r\nGILowPrecision = 1\r\nShadowFmt = 2\r\n";
    let (settings, _) = Settings::read(bytes, 0);
    assert!((settings.aim_speed - 29.0 / 99.0).abs() < 1e-7);
    assert_eq!(settings.perf_index, 1.5);
    assert!(settings.show_fps);
    assert!(!settings.music);
    assert_eq!(settings.screen_width, 1280);
    assert!(settings.gi_low_precision);
    assert_eq!(settings.shadow_format, 2);
    // The graphics rules still apply beside them.
    assert_eq!(settings.shading_quality, 4);
    assert!(settings.vertical_sync);
}
