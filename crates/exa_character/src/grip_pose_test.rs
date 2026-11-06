//! The held-weapon grips against the shipped clips' poses (needs the game
//! installed; skipped without it): where a two-hander's and a polearm's arm
//! link falls from the Idle clip's pose alone.

use bevy::math::{Affine3A, Vec3};
use exa_formats::rsq::{MovesetConfig, Sequences, bone_index};

fn local(node: &exa_formats::rfc::Node) -> Affine3A {
    let t = node.transform;
    let m = bevy::math::Mat3::from_cols(Vec3::from(t[0]), Vec3::from(t[1]), Vec3::from(t[2]));
    Affine3A::from_mat3_translation(m, Vec3::from(t[3]))
}

/// The arm link's span (cm) in `action`'s first pose, with the arm point
/// on bone `arm_bone`: the item placed by its grip under its hand, the
/// table's haft point against the table's arm point.
fn arm_span(
    seq: &Sequences,
    config: MovesetConfig,
    grip: usize,
    action: &str,
    arm_bone: [usize; 2],
) -> f32 {
    let nodes = &seq.skeleton.nodes;
    let clip = seq
        .motions
        .clip(exa_anim::DEFAULT_MOVESET, config, action)
        .expect("the clip");
    let (rotations, root) = exa_anim::sample(clip, 0.0).expect("a pose");
    let mut worlds: Vec<Affine3A> = Vec::with_capacity(nodes.len());
    let mut by_bone = std::collections::HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let mut l = local(node);
        if let Some(bone) = bone_index(&node.name) {
            if let Some(r) = bone.checked_sub(1).and_then(|k| rotations.get(k)) {
                let at = if bone == 1 {
                    root
                } else {
                    l.translation.into()
                };
                l = Affine3A::from_rotation_translation(*r, at);
            }
            by_bone.insert(bone, i);
        }
        worlds.push(node.parent.map_or(l, |p| worlds[p - 1] * l));
    }
    let w = |b: usize| worlds[by_bone[&b]];
    let g = &exa_combat::grip::GRIPS[grip];
    let hold = crate::grip_bone(g.hand);
    let side = usize::from(hold != crate::RIGHT_HAND);
    let place = g.placement(&exa_combat::grip::GripHolder::HUMAN, false);
    let item = w(hold) * Affine3A::from_rotation_translation(place.rotation, place.translation);
    let arm = g.second_hand.as_ref().expect("a two-handed grip").arm;
    let on_item = item.transform_point3(arm.item * 0.01);
    let on_arm = w(arm_bone[side]).transform_point3(arm.part * 0.01);
    (on_item - on_arm).length() * 100.0
}

/// The arm link, which mode 2 (Idle and Walk) enables and relaxes to no
/// slack, ties the haft to the arm chain's root, the shoulder, as the
/// original's link builder does: in the Idle pose a polearm's and a
/// two-hander's arm point is about 12–15 cm from the shoulder's, against
/// 27–30 cm from the forearm's. Relaxed onto the forearm, the link hauled
/// the arm out and toppled a polearm's holder.
#[test]
fn the_arm_link_ties_the_haft_to_the_shoulder() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let game = exa_assets::GameData::open(&dir).unwrap();
    let rsq = game
        .pack(exa_assets::PackKind::Resource)
        .and_then(|p| p.get("humanoid.rsq"))
        .unwrap();
    let seq = Sequences::parse(rsq).unwrap();
    for (config, grip) in [
        (MovesetConfig::POLEARM, 3usize),
        (MovesetConfig::TWO_HANDED, 2),
        (MovesetConfig::GREATSWORD, 2),
    ] {
        let shoulder = arm_span(&seq, config, grip, "Idle", crate::SHOULDER_BONES);
        let forearm = arm_span(&seq, config, grip, "Idle", crate::FOREARM_BONES);
        assert!(
            shoulder < 16.0,
            "{config}: {shoulder:.1} cm from the shoulder"
        );
        assert!(forearm > 25.0, "{config}: {forearm:.1} cm from the forearm");
    }
}
