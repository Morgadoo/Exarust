use super::*;

const VOICED: Sufferer = Sufferer {
    wound_sound: true,
    voice_level: 1.0,
};

#[test]
fn a_deep_wound_groans_and_sounds() {
    let s = pain_sounds(0.5, 0.1, &VOICED);
    let pitch = (f64::from(0.5_f32 * 0.125) + 0.9) as f32;
    let volume = (f64::from(0.5_f32 * 0.5) + 0.2) as f32;
    assert_eq!(
        s.wound,
        Some(Loudness {
            volume: (f64::from(volume) * 0.8) as f32,
            pitch,
        })
    );
    assert_eq!(
        s.voice,
        Some((
            2,
            Loudness {
                volume,
                pitch: (f64::from(pitch) * 0.3 + 0.7) as f32,
            }
        ))
    );
}

#[test]
fn a_lighter_wound_grunts_or_groans_by_the_yellow() {
    // L 0.2: the wound sound plays (past 0.1) at 0.6·L.
    let s = pain_sounds(0.2, 0.4, &VOICED);
    assert_eq!(
        s.wound,
        Some(Loudness {
            volume: (f64::from(0.2_f32) * 0.6) as f32,
            pitch: (f64::from(0.2_f32 * 0.125) + 0.9) as f32,
        })
    );
    let pitch = (f64::from(0.4_f32 * 0.25) + (f64::from(0.2_f32 * 0.5) + 0.7)) as f32;
    let volume = (f64::from(0.4_f32) * 0.3 + (f64::from(0.2_f32) * 0.4 + 0.1)) as f32;
    assert_eq!(
        s.voice,
        Some((
            1,
            Loudness {
                volume,
                pitch: (f64::from(pitch) * 0.3 + 0.7) as f32,
            }
        ))
    );
    // Past a yellow of 0.5 it groans; exactly 0.5 still grunts.
    assert_eq!(pain_sounds(0.2, 0.6, &VOICED).voice.map(|v| v.0), Some(2));
    assert_eq!(pain_sounds(0.2, 0.5, &VOICED).voice.map(|v| v.0), Some(1));
    assert_eq!(
        pain_sounds(0.2, f32::NAN, &VOICED).voice.map(|v| v.0),
        Some(1)
    );
}

#[test]
fn a_bruise_without_a_wound_sound_still_voices() {
    // No lasting wound, yellow 0.3: no wound sound (L not past 0.1), a grunt.
    let s = pain_sounds(0.0, 0.3, &VOICED);
    assert_eq!(s.wound, None);
    assert_eq!(s.voice.map(|v| v.0), Some(1));
    // The float 0.3 widens above the double 0.3: deep. Just under is not.
    assert_eq!(pain_sounds(0.3, 0.0, &VOICED).voice.map(|v| v.0), Some(2));
    let under = f32::from_bits(0.3_f32.to_bits() - 1);
    assert_eq!(pain_sounds(under, 0.0, &VOICED).voice.map(|v| v.0), Some(1));
}

#[test]
fn an_unfelt_blow_or_a_mute_victim_is_quiet() {
    assert_eq!(pain_sounds(0.0, 0.19, &VOICED), PainSounds::default());
    // The float 0.2 widens above the double 0.2: felt.
    assert!(pain_sounds(0.0, 0.2, &VOICED).voice.is_some());
    assert_eq!(pain_sounds(f32::NAN, 0.1, &VOICED), PainSounds::default());
    // A tiny wound is felt: past the epsilon.
    assert!(pain_sounds(1e-6, 0.0, &VOICED).voice.is_some());
    let mute = Sufferer {
        wound_sound: true,
        voice_level: 0.009,
    };
    let s = pain_sounds(0.5, 0.0, &mute);
    assert!(s.wound.is_some());
    assert_eq!(s.voice, None);
    let soundless = Sufferer {
        wound_sound: false,
        voice_level: 1.0,
    };
    assert_eq!(pain_sounds(0.5, 0.0, &soundless).wound, None);
}

#[test]
fn a_struck_player_jolts_by_the_blow() {
    let small = Striker {
        scale: 1.0,
        speed: 20.0,
        overhead: true,
    };
    // 7·0.1 + 7·0.05 = 1.05: past 1, the camera jolts by its root.
    let j = struck_player_jolt(0.05, 0.1, &small);
    let s = 0.1_f32 * 7.0 + 0.05_f32 * 7.0;
    assert_eq!(j.rumble, s);
    assert_eq!(j.camera, Some(s.sqrt()));
    // Under 1 only the pad rumbles.
    assert_eq!(struck_player_jolt(0.0, 0.1, &small).camera, None);
    // A big striker's blow by its speed; overhead at 0.4.
    let big = Striker {
        scale: 1.5,
        ..small
    };
    let s = (f64::from(20.0_f32 * 2.0 + (0.1_f32 * 7.0) * 4.0) * 0.4) as f32;
    assert_eq!(struck_player_jolt(0.0, 0.1, &big).rumble, s);
    let level = Striker {
        overhead: false,
        ..big
    };
    assert_eq!(
        struck_player_jolt(0.0, 0.1, &level).rumble,
        20.0 * 2.0 + (0.1_f32 * 7.0) * 4.0
    );
    // Scale 1.3 exactly is not big (the float widened is above 1.3 as a
    // double only when it rounds up).
    let edge = Striker {
        scale: 1.3,
        ..small
    };
    assert_eq!(
        struck_player_jolt(0.0, 0.1, &edge).rumble > 1.0,
        f64::from(1.3_f32) > 1.3
    );
    assert_eq!(striking_player_rumble(0.6), 0.3);
}
