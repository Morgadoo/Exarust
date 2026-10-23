//! The support pins, pinned to the law (`docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[cfg(test)] #[path = "body_pins_test.rs"] mod pins_test;`)
use super::floor_test::standing_at;
use super::pins::{Factors, pelvis_reach, pin_scale, power, pull, target, thorax_reach};
use super::tests::floor_quad;
use super::*;

/// The law, written out here and not taken from [`pull`]: raw is
/// 0.2·e·|e|/60 under 60 cm and 12 cm along the error otherwise, less the
/// horizontal displacement, capped at 8 cm, times S²·pin-gain·P·pin factor.
/// An alert human is S = 1, pin-gain 0.08, P = 0.5, pin factor 1.25, so
/// the scale is 0.05. A horizontal gap under 1.19e-7 cm² is zero, damper
/// included. Walking and off its feet never reach this function.
fn law(error: Vec3, displacement: Vec3, has_support: bool) -> Vec3 {
    if !has_support {
        return Vec3::ZERO;
    }
    let e = error.with_y(0.0);
    let len2 = e.z * e.z + (e.y * e.y + e.x * e.x);
    if len2 < f32::from_bits(0x33ff_ffd7) {
        return Vec3::ZERO;
    }
    let len = e.length();
    let raw = if len < 60.0 {
        e * (0.2 * len / 60.0)
    } else {
        e * (12.0 / len)
    };
    let mut corr = raw - displacement.with_y(0.0);
    let clen = corr.length();
    if clen > 8.0 {
        corr *= 8.0 / clen;
    }
    let scale = 0.08 * 0.5 * 1.25;
    corr * scale
}

/// The pelvis origin is pulled twice. The second pull sees the first
/// push: the point moves by the sum of the squared weights, which shortens
/// the gap and adds that much to this tick's displacement. `scale` is 2
/// while a shock stagger sets P to 1.
fn pelvis_twice(error: Vec3, displacement: Vec3, weights: [f32; 4], scale: f32) -> Vec3 {
    let first = law(error, displacement, true) * scale;
    let share: f32 = weights.iter().map(|c| c * c).sum();
    let moved = first * share;
    let second = law(error - moved, displacement + moved, true) * scale;
    first + second
}

fn human(error: Vec3, displacement: Vec3, has_support: bool) -> Vec3 {
    pull(
        error,
        displacement,
        Factors {
            has_support,
            support: 1.0,
            pin_gain: 0.08,
            power: 0.5,
            pin_factor: 1.25,
        },
    )
}

/// P is √E·T·E⁴. A NaN or a negative root stays a NaN, and −0 stays −0.
/// The shock-stagger flag replaces it with 1.
#[test]
fn pin_power_is_the_root_unless_the_shock_stagger() {
    assert!((power(1.0, 0.5, false) - 0.5).abs() < 1e-6);
    assert!(power(f32::NAN, 0.5, false).is_nan());
    assert!(power(-1.0, 0.5, false).is_nan());
    assert_eq!(power(-0.0, 0.5, false).to_bits(), (-0.0f32).to_bits());
    assert_eq!(power(0.5, 0.5, true), 1.0);
    assert_eq!(power(f32::NAN, 0.5, true), 1.0);
}

/// One application equals the law: 10 cm off with 1 cm of displacement
/// along the error is −0.0333 cm; 80 cm with none is 0.4 cm; no support
/// is zero.
#[test]
fn one_application_is_the_law() {
    let near = Vec3::X * 10.0;
    let along = Vec3::X;
    let got = human(near, along, true);
    assert!(got.abs_diff_eq(law(near, along, true), 1e-6), "{got}");
    assert!((got.x + 0.0333333).abs() < 1e-4, "{got}");
    assert_eq!((got.y, got.z), (0.0, 0.0));

    let far = Vec3::new(48.0, 9.0, 64.0);
    assert!(far.length() > 60.0);
    let got = human(far, Vec3::ZERO, true);
    let want = law(far, Vec3::ZERO, true);
    assert!(got.abs_diff_eq(want, 1e-5), "{got} vs {want}");
    assert!((got.length() - 0.4).abs() < 1e-5, "{got}");

    let along_x = Vec3::X * 80.0;
    let got = human(along_x, Vec3::ZERO, true);
    assert!(
        (got.x - 0.4).abs() < 1e-5 && got.y == 0.0 && got.z == 0.0,
        "{got}"
    );

    assert_eq!(human(near, along, false), Vec3::ZERO);
}

/// The gains multiply in the original's order: P = (√E·T)·((E·E)·(E·E))
/// groups E⁴ before the tone, and the pin scale is S²·(G·(P·F)). For
/// E = 0.7, T = 0.8, P = 0.16070566; for S = 0.77, G = 0.093, P = 0.61,
/// F = 0.83, the scale is 0.027917228 (the left-to-right product rounds
/// to 0.027917231).
#[test]
fn the_pin_products_group_as_the_original_does() {
    assert_eq!(power(0.7, 0.8, false), 0.160_705_66);
    let gain = Factors {
        has_support: true,
        support: 0.77,
        pin_gain: 0.093,
        power: 0.61,
        pin_factor: 0.83,
    };
    assert_eq!(pin_scale(gain), 0.027_917_228);
}

/// A horizontal gap under 1.19e-7 cm² leaves the pin. The damper does not
/// run either: 1 cm of displacement on a zero gap, or on a gap whose square
/// is under the epsilon, is zero. A NaN gap is zero. A gap just over the
/// epsilon still damps.
#[test]
fn a_tiny_gap_skips_the_damper() {
    let moved = Vec3::X;
    assert_eq!(human(Vec3::ZERO, moved, true), Vec3::ZERO);
    assert_eq!(law(Vec3::ZERO, moved, true), Vec3::ZERO);

    let eps = f32::from_bits(0x33ff_ffd7);
    let under = Vec3::X * (eps.sqrt() * 0.5);
    assert!(under.x * under.x < eps, "{}", under.x * under.x);
    assert_eq!(human(under, moved, true), Vec3::ZERO);
    assert_eq!(law(under, moved, true), Vec3::ZERO);

    assert_eq!(
        human(Vec3::new(f32::NAN, 0.0, 0.0), moved, true),
        Vec3::ZERO
    );

    let over = Vec3::X * 0.001;
    assert!(over.x * over.x > eps);
    let got = human(over, moved, true);
    assert!(got.abs_diff_eq(law(over, moved, true), 1e-6), "{got}");
    assert!(got.x < -0.04, "{got}");
}

/// Thorax targets stay within 36·scale cm of the feet centre standing
/// (crouch 0), 18 cm at a full crouch while stepping, and the pelvis pin
/// within 60·scale cm. With no support the target only shifts 5·scale cm
/// forward.
#[test]
fn targets_stay_near_the_feet_and_shift_forward_without_support() {
    let centre = Vec3::new(1.0, 0.2, -2.0);
    let posed = centre + Vec3::new(0.40, 1.2, 0.0);
    let standing = thorax_reach(0.0, false, 1.0);
    assert!((standing - 36.0).abs() < 1e-5, "{standing}");
    let t = target(posed, centre, standing, true, Vec3::Z, 1.0);
    assert!((t.x - (centre.x + 0.36)).abs() < 1e-5, "{t}");
    assert_eq!(t.y, posed.y);

    let pelvis = target(
        centre + Vec3::new(0.0, 1.1, 0.80),
        centre,
        pelvis_reach(1.0),
        true,
        Vec3::Z,
        1.0,
    );
    assert!((pelvis.z - (centre.z + 0.60)).abs() < 1e-5, "{pelvis}");

    let stepping = thorax_reach(1.0, true, 1.0);
    assert!((stepping - 18.0).abs() < 1e-5, "{stepping}");
    let wide = centre + Vec3::X * 0.40;
    let t = target(wide, centre, stepping, true, Vec3::Z, 1.0);
    assert!((t.x - (centre.x + 0.18)).abs() < 1e-5, "{t}");

    let ahead = target(posed, centre, standing, false, Vec3::Z, 1.0);
    assert!(ahead.abs_diff_eq(posed + Vec3::Z * 0.05, 1e-6), "{ahead}");
    let tall = target(posed, centre, standing, false, Vec3::Z, 2.0);
    assert!(tall.abs_diff_eq(posed + Vec3::Z * 0.10, 1e-6), "{tall}");
}

fn shift(part: &mut Part, by: Vec3) {
    for x in part
        .positions
        .iter_mut()
        .chain(part.previous.iter_mut())
        .chain(part.start.iter_mut())
    {
        *x += by;
    }
}

fn feet_centre(body: &Ragdoll) -> Vec3 {
    let n = body.legs().len() as f32;
    (0..body.legs().len())
        .map(|i| body.ankle(i).unwrap())
        .sum::<Vec3>()
        / n
}

/// The clamp the spec states, not [`target`].
fn clamped(posed: Vec3, centre: Vec3, radius_cm: f32) -> Vec3 {
    let mut d = (posed - centre).with_y(0.0);
    let len = d.length() * 100.0;
    if radius_cm <= 0.0 {
        d = Vec3::ZERO;
    } else if len > radius_cm {
        d *= radius_cm / len;
    }
    Vec3::new(centre.x + d.x, posed.y, centre.z + d.z)
}

/// One pass on a real body moves each pin's particles by the law and
/// nothing else. The thorax is 10 cm off its pose, so each thorax pin's
/// error is 10 cm and its displacement is zero.
#[test]
fn one_pass_on_a_body_applies_the_law() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    body.aim(Affine3A::IDENTITY, pose);
    let thorax = body.parts.iter().position(|p| p.bone == 8).unwrap();
    shift(&mut body.parts[thorax], Vec3::X * 0.10);
    let centre = feet_centre(&body);
    let targets = body.drive.as_ref().unwrap().targets().to_vec();
    let mut pins: Vec<(usize, [f32; 4], f32)> = body
        .chest
        .iter()
        .copied()
        .map(|(i, w)| (i, w, 36.0))
        .collect();
    let pelvis = body.parts.iter().position(|p| p.bone == 1).unwrap();
    pins.push((pelvis, body.parts[pelvis].own_origin, 60.0));
    let mut expected = vec![[Vec3::ZERO; 4]; body.parts.len()];
    for (i, w, radius) in pins {
        let posed = targets[i].transform_point3(body.parts[i].rest_point(&w));
        let goal = clamped(posed, centre, radius);
        let at = body.parts[i].point(&w);
        let disp = (0..4)
            .map(|k| (body.parts[i].positions[k] - body.parts[i].start[k]) * w[k])
            .sum::<Vec3>();
        let error = ((goal - at).with_y(0.0)) * 100.0;
        let moved = disp.with_y(0.0) * 100.0;
        let corr = if i == pelvis {
            pelvis_twice(error, moved, w, 1.0)
        } else {
            law(error, moved, true)
        };
        for k in 0..4 {
            expected[i][k] += corr / 100.0 * w[k];
        }
    }
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    body.pull_support_pins();
    for (i, p) in body.parts.iter().enumerate() {
        for k in 0..4 {
            let delta = p.positions[k] - before[i][k];
            assert!(
                delta.abs_diff_eq(expected[i][k], 1e-6),
                "part {i} particle {k}: {delta} vs {}",
                expected[i][k]
            );
        }
    }
}

/// The pelvis origin is pulled a second time after the first push. A
/// thorax pin in the same pass is pulled once. Shifting both 10 cm, with
/// no displacement yet, the pelvis particles move by the two sequential
/// applications and the thorax particles by one.
#[test]
fn the_pelvis_origin_is_pulled_twice() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    body.aim(Affine3A::IDENTITY, pose);
    let thorax = body.parts.iter().position(|p| p.bone == 8).unwrap();
    let pelvis = body.parts.iter().position(|p| p.bone == 1).unwrap();
    shift(&mut body.parts[thorax], Vec3::X * 0.10);
    shift(&mut body.parts[pelvis], Vec3::X * 0.10);
    let centre = feet_centre(&body);
    let targets = body.drive.as_ref().unwrap().targets().to_vec();
    let mut expected = vec![[Vec3::ZERO; 4]; body.parts.len()];
    for (i, w, radius) in
        body.chest
            .iter()
            .copied()
            .map(|(i, w)| (i, w, 36.0))
            .chain(std::iter::once((
                pelvis,
                body.parts[pelvis].own_origin,
                60.0,
            )))
    {
        let posed = targets[i].transform_point3(body.parts[i].rest_point(&w));
        let goal = clamped(posed, centre, radius);
        let at = body.parts[i].point(&w);
        let error = ((goal - at).with_y(0.0)) * 100.0;
        let corr = if i == pelvis {
            pelvis_twice(error, Vec3::ZERO, w, 1.0)
        } else {
            law(error, Vec3::ZERO, true)
        };
        for k in 0..4 {
            expected[i][k] += corr / 100.0 * w[k];
        }
    }
    let once = {
        let w = body.parts[pelvis].own_origin;
        let posed = targets[pelvis].transform_point3(body.parts[pelvis].rest_point(&w));
        let goal = clamped(posed, centre, 60.0);
        let at = body.parts[pelvis].point(&w);
        law(((goal - at).with_y(0.0)) * 100.0, Vec3::ZERO, true)
    };
    let twice = pelvis_twice(
        {
            let w = body.parts[pelvis].own_origin;
            let posed = targets[pelvis].transform_point3(body.parts[pelvis].rest_point(&w));
            let goal = clamped(posed, centre, 60.0);
            let at = body.parts[pelvis].point(&w);
            ((goal - at).with_y(0.0)) * 100.0
        },
        Vec3::ZERO,
        body.parts[pelvis].own_origin,
        1.0,
    );
    assert!(
        (twice.length() - once.length()).abs() > 1e-4,
        "the second pull matched the first: {twice} vs {once}"
    );
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    body.pull_support_pins();
    for (i, p) in body.parts.iter().enumerate() {
        for k in 0..4 {
            let delta = p.positions[k] - before[i][k];
            assert!(
                delta.abs_diff_eq(expected[i][k], 1e-6),
                "part {i} particle {k}: {delta} vs {}",
                expected[i][k]
            );
        }
    }
}

/// A shock stagger uses P = 1, twice an alert human's √1·0.5·1⁴.
#[test]
fn a_shock_stagger_pulls_the_pins_at_full_power() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    body.aim(Affine3A::IDENTITY, pose);
    let thorax = body.parts.iter().position(|p| p.bone == 8).unwrap();
    shift(&mut body.parts[thorax], Vec3::X * 0.10);
    body.drive.as_mut().unwrap().bearing.shock_stagger = true;
    let centre = feet_centre(&body);
    let targets = body.drive.as_ref().unwrap().targets().to_vec();
    let mut pins: Vec<(usize, [f32; 4], f32)> = body
        .chest
        .iter()
        .copied()
        .map(|(i, w)| (i, w, 36.0))
        .collect();
    let pelvis = body.parts.iter().position(|p| p.bone == 1).unwrap();
    pins.push((pelvis, body.parts[pelvis].own_origin, 60.0));
    // Each pin applied to a copy in sequence, as the pass does: the second
    // thorax pin reads the first's push. P is 1, so each correction is
    // twice the alert human's law.
    let mut sim = body.clone();
    let mut expected = vec![[Vec3::ZERO; 4]; body.parts.len()];
    let pin = |sim: &mut Ragdoll, i: usize, w: [f32; 4], radius: f32| {
        let posed = targets[i].transform_point3(sim.parts[i].rest_point(&w));
        let goal = clamped(posed, centre, radius);
        let at = sim.parts[i].point(&w);
        let disp = (0..4)
            .map(|k| (sim.parts[i].positions[k] - sim.parts[i].start[k]) * w[k])
            .sum::<Vec3>();
        let error = ((goal - at).with_y(0.0)) * 100.0;
        let moved = disp.with_y(0.0) * 100.0;
        law(error, moved, true) * 2.0
    };
    for (i, w, radius) in pins.iter().copied() {
        let corr = pin(&mut sim, i, w, radius);
        sim.parts[i].push(&w, corr / 100.0);
        for k in 0..4 {
            expected[i][k] += corr / 100.0 * w[k];
        }
    }
    // The pelvis origin's second pull, after the first.
    let w = body.parts[pelvis].own_origin;
    let corr = pin(&mut sim, pelvis, w, 60.0);
    sim.parts[pelvis].push(&w, corr / 100.0);
    for k in 0..4 {
        expected[pelvis][k] += corr / 100.0 * w[k];
    }
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    body.pull_support_pins();
    for (i, p) in body.parts.iter().enumerate() {
        for k in 0..4 {
            let delta = p.positions[k] - before[i][k];
            assert!(
                delta.abs_diff_eq(expected[i][k], 1e-6),
                "part {i} particle {k}: {delta} vs {}",
                expected[i][k]
            );
        }
    }
}

fn stand(body: &mut Ragdoll, pose: &[Affine3A], level: &[[Vec3; 3]]) {
    body.measure_balance(level);
    body.aim(Affine3A::IDENTITY, pose.to_vec());
    body.tick(level);
}

fn chest_off_feet(body: &Ragdoll) -> f32 {
    let chest = body.chest_point().unwrap();
    (chest - feet_centre(body)).with_y(0.0).length() * 100.0
}

/// Walking, or off its feet (down, getting up, dormant or dead), the pin
/// loops do not run: a chest held 10 cm off the feet centre stays there.
/// A shock stagger does not reopen them. Collapsing is not that condition,
/// so the standing law still applies.
#[test]
fn walking_or_off_its_feet_the_pins_stay() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    body.aim(Affine3A::IDENTITY, pose);
    let thorax = body.parts.iter().position(|p| p.bone == 8).unwrap();
    shift(&mut body.parts[thorax], Vec3::X * 0.10);

    let stay = |body: &mut Ragdoll, bearing: Bearing| {
        body.drive.as_mut().unwrap().bearing = bearing;
        let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
        body.pull_support_pins();
        for (i, p) in body.parts.iter().enumerate() {
            assert_eq!(p.positions, before[i], "part {i} moved");
        }
    };
    stay(
        &mut body,
        Bearing {
            walking: true,
            ..Bearing::default()
        },
    );
    stay(
        &mut body,
        Bearing {
            off_feet: true,
            ..Bearing::default()
        },
    );
    stay(
        &mut body,
        Bearing {
            off_feet: true,
            getting_up: true,
            ..Bearing::default()
        },
    );
    stay(
        &mut body,
        Bearing {
            walking: true,
            shock_stagger: true,
            ..Bearing::default()
        },
    );

    body.drive.as_mut().unwrap().bearing = Bearing {
        collapsing: true,
        ..Bearing::default()
    };
    let centre = feet_centre(&body);
    let targets = body.drive.as_ref().unwrap().targets().to_vec();
    let mut pins: Vec<(usize, [f32; 4], f32)> = body
        .chest
        .iter()
        .copied()
        .map(|(i, w)| (i, w, 36.0))
        .collect();
    let pelvis = body.parts.iter().position(|p| p.bone == 1).unwrap();
    pins.push((pelvis, body.parts[pelvis].own_origin, 60.0));
    let mut expected = vec![[Vec3::ZERO; 4]; body.parts.len()];
    for (i, w, radius) in pins {
        let posed = targets[i].transform_point3(body.parts[i].rest_point(&w));
        let goal = clamped(posed, centre, radius);
        let at = body.parts[i].point(&w);
        let disp = (0..4)
            .map(|k| (body.parts[i].positions[k] - body.parts[i].start[k]) * w[k])
            .sum::<Vec3>();
        let error = ((goal - at).with_y(0.0)) * 100.0;
        let moved = disp.with_y(0.0) * 100.0;
        let corr = if i == pelvis {
            pelvis_twice(error, moved, w, 1.0)
        } else {
            law(error, moved, true)
        };
        for k in 0..4 {
            expected[i][k] += corr / 100.0 * w[k];
        }
    }
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    body.pull_support_pins();
    for (i, p) in body.parts.iter().enumerate() {
        for k in 0..4 {
            let delta = p.positions[k] - before[i][k];
            assert!(
                delta.abs_diff_eq(expected[i][k], 1e-6),
                "part {i} particle {k}: {delta} vs {}",
                expected[i][k]
            );
        }
    }
}

/// A supported chest held about 10 cm off the feet centre is brought back
/// toward the feet over later ticks of the real step. The same offset with
/// no support is not pulled back by the pins.
#[test]
fn a_chest_ten_cm_off_the_feet_comes_back() {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = standing_at(Vec3::ZERO);
    for _ in 0..30 {
        stand(&mut body, &pose, &floor);
    }
    let thorax = body.parts.iter().position(|p| p.bone == 8).unwrap();
    let by = Vec3::X * 0.10;
    for k in 0..4 {
        body.parts[thorax].positions[k] += by;
        body.parts[thorax].previous[k] += by;
    }
    let held = chest_off_feet(&body);
    assert!(held > 8.0 && held < 12.0, "held {held} cm off");
    for _ in 0..80 {
        stand(&mut body, &pose, &floor);
    }
    let back = chest_off_feet(&body);
    assert!(
        back < held - 1.0,
        "the chest closed only {} cm ({held} to {back})",
        held - back
    );

    let (mut loose, pose) = standing_at(Vec3::ZERO);
    loose.aim(Affine3A::IDENTITY, pose);
    loose.drive.as_mut().unwrap().support = 0.0;
    // No floor under it: the balance measure leaves it without support.
    loose.measure_balance(&[] as &[[Vec3; 3]]);
    assert!(!loose.drive.as_ref().unwrap().balance().has_support());
    let thorax = loose.parts.iter().position(|p| p.bone == 8).unwrap();
    shift(&mut loose.parts[thorax], by);
    let before: Vec<[Vec3; 4]> = loose.parts.iter().map(|p| p.positions).collect();
    loose.pull_support_pins();
    for (i, p) in loose.parts.iter().enumerate() {
        assert_eq!(p.positions, before[i], "part {i} moved without support");
    }
}

/// While the body moves, the frame follows the pelvis, so the pins do not
/// nail it to the world spot it started on.
#[test]
fn a_moving_body_is_not_nailed_to_its_spot() {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = standing_at(Vec3::ZERO);
    for _ in 0..20 {
        stand(&mut body, &pose, &floor);
    }
    let start = Vec3::from(body.part_of(1).unwrap().transform().translation);
    let anchor0 = body.drive.as_ref().unwrap().anchor().unwrap();
    body.set_bearing(Bearing {
        moving: true,
        ..Bearing::default()
    });
    body.set_velocity(Vec3::X * 1.5);
    let (mut far, mut frame_far) = (0.0f32, 0.0f32);
    for _ in 0..40 {
        body.set_bearing(Bearing {
            moving: true,
            ..Bearing::default()
        });
        stand(&mut body, &pose, &floor);
        let end = Vec3::from(body.part_of(1).unwrap().transform().translation);
        far = far.max((end - start).with_y(0.0).length() * 100.0);
        let frame = (body.drive.as_ref().unwrap().anchor().unwrap() - anchor0)
            .with_y(0.0)
            .length()
            * 100.0;
        frame_far = frame_far.max(frame);
    }
    // The planted feet still grip, so the body does not coast at 1.5 m/s.
    // It does leave its spot, and the frame goes with the pelvis, so the
    // pin targets are not left on the world spot it started on.
    assert!(
        far > 3.0,
        "the pelvis stayed within {far} cm of where it started"
    );
    assert!(
        frame_far > 2.0,
        "the frame stayed within {frame_far} cm, so the pins still aimed at the start"
    );
}
