//! The links that hold a held item's body to the hand that holds it, and
//! the grip's modes.
//!
//! The original keeps a held weapon, shield or torch as its own rigid body
//! (`TTetraRBD`) whose parent is the hand's body. Its first link is the
//! pivot at the item's origin; the grip table's points become links from
//! the item's rest particles to the hand's (and, for a two-handed or
//! polearm grip, to the other hand and the arm). This module builds those
//! link records and switches them as the grip's modes do; it is plain data
//! so the body solver can apply a link to its item's and its parent's
//! particles (`GripLink::apply` documents the law). `grip_solve` runs the
//! records as a cross-body stage: a character's equip and torch put a
//! `HeldGrip` on an item whose model has a rigid root, and its item body is
//! pulled to the hand every solver iteration. A factory weapon with no
//! stored rigid node stays bone-attached.
//!
//! The numbers are the original's, in its centimetres and flags. Evidence:
//! the grip table, the link builder (points scaled by the body's scale,
//! the second hand's by its size and W24, the extra and flip points raw),
//! the two grip callbacks, the relax pass and the flip animation.

use bevy::math::{Affine3A, Vec3};
use bevy::prelude::{Component, Entity};

use crate::body::{Part, Ragdoll};

/// A link's pivot: a point on the item tied to its parent with the slack's
/// range.
pub const PIVOT: u32 = 0x1_0000;
/// A link that was enabled at least once (the relax pass reads it).
pub const ENABLED: u32 = 0x100;
/// A disabled link: the solver skips it.
pub const DISABLED: u32 = 0x10_0000;
/// A horizontal-only link (the original's first two iterations only).
pub const HORIZONTAL: u32 = 0x200;
/// A runtime (world or grip) link, not one from the model file.
pub const RUNTIME: u32 = 0x40_0000;
/// The smoothing every runtime link starts with when the builder leaves it
/// at zero (the link init's 0.001).
pub const DEFAULT_SMOOTHING: f32 = 0.001;

/// A point on the item in its own frame (cm) and the slack (cm) its link
/// allows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GripPoint {
    pub point: Vec3,
    pub slack: f32,
}

/// A point on the item (cm) tied to a point on another body part (cm, in
/// that part's frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GripPair {
    pub item: Vec3,
    pub part: Vec3,
}

/// The part of the grip table the link builder reads.
#[derive(Debug, Clone, PartialEq)]
pub struct GripSpec {
    /// The holding hand: 0 right, 1 left (the table's entry `[2]`).
    pub hand: usize,
    /// The main grip links, in the table's order.
    pub links: Vec<GripPoint>,
    /// The second hand's link (item point, other hand point).
    pub second_hand: Option<GripPair>,
    /// The arm's link (item point, own arm point).
    pub arm: Option<GripPair>,
    /// The extra link a two-hander adds while stabbing (item point, other
    /// hand point).
    pub extra: Option<GripPair>,
}

impl GripSpec {
    /// The callback the original's table stores for this entry: the
    /// two-handed one when both a second-hand and an extra link exist, the
    /// polearm one with only a second-hand link, none otherwise.
    pub fn kind(&self) -> GripKind {
        match (self.second_hand.is_some(), self.extra.is_some()) {
            (true, true) => GripKind::TwoHanded,
            (true, false) => GripKind::Polearm,
            _ => GripKind::Other,
        }
    }
}

/// Which grip callback switches the links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GripKind {
    /// A two-hander: the second hand, the arm and the stab's extra link.
    TwoHanded,
    /// A polearm: the main link slides, the second hand and the arm.
    Polearm,
    /// A one-handed item, a light or a shield: the main links only.
    Other,
}

/// What a held item is, as its grip is built: the table's weapon links, a
/// shield's three face pivots, or a torch's own two pivots. A torch and a
/// shield always carry their own soft world weld.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeldKind {
    /// A weapon, from the grip table.
    #[default]
    Weapon,
    /// A shield on the off hand.
    Shield,
    /// A torch in a hand.
    Torch,
}

/// The parts of the two bodies a grip joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GripBindings {
    /// The item's body part the links attach to (its root).
    pub item_part: usize,
    /// The character's hand parts, right then left.
    pub hand_parts: [usize; 2],
    /// The character's forearms, right then left.
    pub arm_parts: [usize; 2],
}

/// One grip link: a point on the item's part tied to a point on the hand's
/// part (or to the world). The law is the original's: the gap `d` between
/// the two points is shared by the two mobilities,
/// `item.push(on_item, c·shares.0)` and `hand.push(on_hand, −c·shares.1)`,
/// where `c` is the link's correction (a pivot's softened gap, a slack
/// link's excess). The shares are already the mobility proportions.
#[derive(Debug, Clone, PartialEq)]
pub struct GripLink {
    /// The item's part.
    pub item: usize,
    /// The weights on the item part's rest particles.
    pub on_item: [f32; 4],
    /// The hand's part; `None` is the world anchor.
    pub hand: Option<usize>,
    /// The weights on the hand part's rest particles.
    pub on_hand: [f32; 4],
    /// The link's slack (m).
    pub slack: f32,
    /// The smoothed length (m) a limp link holds inside its slack.
    pub length: f32,
    pub strength: f32,
    pub smoothing: f32,
    /// The original's link flags ([`PIVOT`], [`DISABLED`], …).
    pub flags: u32,
    /// The item's and the parent's shares of a correction (both positive;
    /// the parent is pushed the other way).
    pub shares: (f32, f32),
    /// The world point a link with no parent part holds to, set when the
    /// link is built.
    pub anchor: Option<Vec3>,
}

impl GripLink {
    /// The link's correction is applied unless it is disabled.
    pub fn enabled(&self) -> bool {
        self.flags & DISABLED == 0
    }

    /// It was enabled at least once: the relax pass takes it.
    pub fn once_enabled(&self) -> bool {
        self.flags & ENABLED != 0
    }

    /// Enables the link (it takes the enabled bit and loses the disabled one) and
    /// gives it the slack `slack` (m).
    pub fn enable(&mut self, slack: f32) {
        self.flags = (self.flags & !DISABLED) | ENABLED;
        self.slack = slack;
        self.length = slack;
    }

    /// Disables the link (it takes the disabled bit).
    pub fn disable(&mut self) {
        self.flags |= DISABLED;
    }
}

/// The links of a held item and the grip's state.
#[derive(Debug, Clone, PartialEq)]
pub struct GripLinks {
    pub links: Vec<GripLink>,
    /// The main grip link: the last of the table's links.
    pub main: usize,
    /// The second hand's link.
    pub second: Option<usize>,
    /// The arm's link.
    pub arm: Option<usize>,
    /// The stab's extra link.
    pub extra: Option<usize>,
    /// The link the flip turns.
    pub flip: Option<usize>,
    /// The point the flip turns about, in the item's frame (cm).
    pub flip_point: Vec3,
    /// The flip's progress, 0 to 1.
    pub flip_progress: f32,
    /// Whether the item is held flipped in its current pose (the original's
    /// item flag): a flip runs from this state to its opposite.
    pub flipped: bool,
    /// The original's flip-active flag: the animation runs only while this
    /// is set, and clears it when the progress reaches an end.
    pub flip_active: bool,
    /// The grip mode, -1 before the first callback.
    pub mode: i32,
    /// A link still relaxing after a grip change.
    pub relaxing: bool,
    /// The flip finished and the item's record must be rebuilt.
    pub record_dirty: bool,
    /// The soft world link and the stiff one.
    pub world_soft: Option<usize>,
    pub world_hard: Option<usize>,
}

/// A held item its holder let go of: the collapse of a living body drops
/// its weapons and torch (a shield stays on the arm). The grip stage frees
/// the item's links and its body.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GripRelease;

/// A held item waiting for its grip links, or holding them. The physics
/// side of an item held in a hand: the item's own body is a [`Ragdoll`]
/// whose part 0 is the item's root, and the character's body holds the
/// hands and arms the links tie it to. A system builds `links` once both
/// bodies exist; [`crate::grip_solve::GripSolver`] then pulls the item to
/// the hand every iteration.
#[derive(Component, Clone, Debug)]
pub struct HeldGrip {
    /// The grip table entry's links.
    pub spec: GripSpec,
    /// The character's body scale, which scales the main points.
    pub scale: f32,
    /// The character's size, which scales the second hand's points.
    pub size: f32,
    /// The weapon record's second-hand factor (W24).
    pub second_hand_factor: f32,
    /// The item is held flipped.
    pub flip: bool,
    /// The item's weapon class (the low 16 bits of its weapon section): 0
    /// for a light's own class word, which places a two-hander's soft weld
    /// at the grip instead of up the box.
    pub class: u32,
    /// The soft (parry) world weld's gate: the original's auto-parry
    /// flag, set for a holder that may parry (the global parry switch is
    /// on, the heavy-claws state and race flag bit 0 are clear). Without
    /// it the item gets the stiff origin weld alone. A shield or a torch
    /// always gets its own soft weld.
    pub soft_weld: bool,
    /// What the item is: a weapon, a shield or a torch.
    pub kind: HeldKind,
    /// The body of the character holding it, as the pair stage knows it
    /// (its first part's entity).
    pub holder_key: Entity,
    /// The hand parts, right then left.
    pub hands: [Option<Entity>; 2],
    /// The forearms, right then left.
    pub arms: [Option<Entity>; 2],
    /// The built links, once both bodies exist.
    pub links: Option<GripLinks>,
    /// The holder let go (died or collapsed): the links are not rebuilt.
    pub released: bool,
}

impl HeldGrip {
    /// The item body's part index the links attach to.
    pub const ITEM_PART: usize = 0;
}

/// A soft world weld's strength.
pub const SOFT_WORLD: f32 = 0.05;
/// A stiff world weld's strength.
pub const STIFF_WORLD: f32 = 1.0;
/// The link mobility a free hand has, as a share of its base mass: the
/// original's `0.175`.
pub const HAND_LINK_SHARE: f32 = 0.175;

/// An empty hand's link mobility and mass words from its base mass, as the
/// original restores them when the hands are emptied: the mobility is the
/// base mass times 0.175 and the mass is the base mass unchanged.
pub fn empty_hand_words(base_mass: f32) -> (f32, f32) {
    (base_mass * HAND_LINK_SHARE, base_mass * 1.0)
}

/// An unarmed hand's mobility as a share of its base: a half, a quarter for
/// a race with heavy claws (its hands weigh 2 or 4 times their part, as
/// [`exa_formats::weapon_record::Equipped::hand_mass`] says).
pub const UNARMED_HAND_SHARE: f32 = 0.5;
/// The same for heavy claws.
pub const HEAVY_CLAW_HAND_SHARE: f32 = 0.25;

/// The mobility word the equip pass gives each empty hand when the
/// character holds no weapon (its class has no hands bits): the hand's base
/// times [`UNARMED_HAND_SHARE`], or [`HEAVY_CLAW_HAND_SHARE`] with heavy
/// claws. It overwrites the mass word [`empty_hand_words`] restored; the
/// link mobility word keeps its 0.175 share. Single-precision products, as
/// the original. Exarust's hand parts have no separate mobility word yet,
/// so no body reads it.
pub fn unarmed_hand_mobility(base_mass: f32, heavy_claws: bool) -> f32 {
    base_mass
        * if heavy_claws {
            HEAVY_CLAW_HAND_SHARE
        } else {
            UNARMED_HAND_SHARE
        }
}

/// The distance (m) between a link's two points now.
fn link_distance(link: &GripLink, item: &Ragdoll, hand: &Ragdoll) -> Option<f32> {
    let a = item.point_at(link.item, &link.on_item)?;
    let b = link
        .hand
        .and_then(|part| hand.point_at(part, &link.on_hand))?;
    Some(a.distance(b))
}

/// The shares of a correction from the item's and the parent's mobilities.
fn shares(item_mobility: f32, parent_mobility: f32) -> (f32, f32) {
    let sum = item_mobility + parent_mobility;
    if sum > 0.0 && sum.is_finite() {
        (item_mobility / sum, parent_mobility / sum)
    } else {
        (0.5, 0.5)
    }
}

/// A grip link's weights and shares.
#[allow(clippy::too_many_arguments)]
fn make_link(
    item: &Ragdoll,
    hand: &Ragdoll,
    bindings: GripBindings,
    parent: Option<usize>,
    on_item: Vec3,
    on_hand: Vec3,
    slack: f32,
    strength: f32,
    smoothing: f32,
    flags: u32,
) -> Option<GripLink> {
    let on_item = item.weights_at(bindings.item_part, on_item)?;
    let on_hand = hand.weights_at(parent?, on_hand)?;
    let item_mobility = item.parts.get(bindings.item_part)?.inverse_mass;
    let parent_mobility = hand.link_mobility(parent?)?;
    Some(GripLink {
        item: bindings.item_part,
        on_item,
        hand: parent,
        on_hand,
        slack,
        length: slack,
        strength,
        // The link init turns a zero smoothing into 0.001.
        smoothing: if smoothing == 0.0 {
            DEFAULT_SMOOTHING
        } else {
            smoothing
        },
        flags,
        shares: shares(item_mobility, parent_mobility),
        anchor: None,
    })
}

/// Builds the grip links of an item body held by a hand body, in the
/// original's order: the second hand's and the arm's links first (built
/// disabled), then the stab's extra link (disabled), then the table's main
/// links. `scale` is the character's body scale (the table's
/// points are scaled by it), `size` its size (which scales the
/// second hand's and the arm's points), `item_to_hand` the item node's
/// matrix in the hand's frame (m), `flip` whether the item is turned, and
/// `second_hand_factor` W24. The hand/arm points are the table's own points
/// in their parts' frames. `None` when a part or point is malformed.
#[allow(clippy::too_many_arguments)]
pub fn build(
    item: &Ragdoll,
    hand: &Ragdoll,
    bindings: GripBindings,
    spec: &GripSpec,
    scale: f32,
    size: f32,
    item_to_hand: Affine3A,
    flip: bool,
    second_hand_factor: f32,
) -> Option<GripLinks> {
    let holding = bindings.hand_parts.get(spec.hand).copied()?;
    let other = bindings.hand_parts.get(1 - spec.hand).copied()?;
    let holding_arm = bindings.arm_parts.get(spec.hand).copied()?;
    let mut links = Vec::new();
    let mut second = None;
    let mut arm = None;
    let mut extra = None;
    let cm = 0.01;
    if let Some(pair) = spec.second_hand {
        let mut point = pair.item * size * cm;
        point.y *= second_hand_factor;
        let link = make_link(
            item,
            hand,
            bindings,
            Some(other),
            point,
            pair.part * size * cm,
            0.0,
            1.0,
            0.0,
            PIVOT | DISABLED,
        )?;
        second = Some(links.len());
        links.push(link);
        let link = make_link(
            item,
            hand,
            bindings,
            Some(holding_arm),
            spec.arm?.item * size * cm,
            // The arm's hand-side point is the table's raw point (the
            // second hand's alone is scaled by the character's size).
            spec.arm?.part * cm,
            0.0,
            1.0,
            0.0,
            PIVOT | DISABLED,
        )?;
        arm = Some(links.len());
        links.push(link);
    }
    if let Some(pair) = spec.extra {
        let link = make_link(
            item,
            hand,
            bindings,
            Some(other),
            pair.item * cm,
            pair.part * cm,
            0.0,
            1.0,
            0.0,
            PIVOT | DISABLED,
        )?;
        extra = Some(links.len());
        links.push(link);
    }
    let mut main = 0;
    for (i, grip) in spec.links.iter().enumerate() {
        let mut point = grip.point * scale * cm;
        if flip {
            // The flipped weapon mirrors x and z (the original's sign-bit
            // XOR on the first and third coordinates).
            point.x = -point.x;
            point.z = -point.z;
        }
        let on_hand = item_to_hand.transform_point3(point);
        let link = make_link(
            item,
            hand,
            bindings,
            Some(holding),
            point,
            on_hand,
            grip.slack * cm,
            1.0,
            if i == 1 { 0.01 } else { 0.0 },
            PIVOT,
        )?;
        main = links.len();
        links.push(link);
    }
    let flip_link = if spec.links.is_empty() {
        None
    } else if spec.second_hand.is_some() {
        (main > 0).then_some(main - 1)
    } else {
        Some(main)
    };
    // The table's entries carry at least two main links; a one-link spec
    // (not shipped) takes its only point, where the original would read
    // the next slot.
    let flip_point = if spec.second_hand.is_some() {
        spec.links.first()?.point * scale
    } else {
        spec.links.get(1).or_else(|| spec.links.first())?.point * scale
    };
    Some(GripLinks {
        links,
        main,
        second,
        arm,
        extra,
        flip: flip_link,
        flip_point,
        flip_progress: if flip { 1.0 } else { 0.0 },
        flipped: flip,
        flip_active: false,
        mode: -1,
        relaxing: false,
        record_dirty: false,
        world_soft: None,
        world_hard: None,
    })
}

/// Adds the two world links a held item gets when it is attached
///: a soft one at `soft_point` (cm) with strength 0.05 and a
/// stiff one at the item's origin with strength 1, both to the world
/// anchor, flags `0x500000`. `None` when the item part does not exist.
pub fn world_links(
    item: &Ragdoll,
    bindings: GripBindings,
    soft_point: Vec3,
    soft: bool,
) -> Option<Vec<GripLink>> {
    let cm = 0.01;
    let on_item = item.weights_at(bindings.item_part, soft_point * cm)?;
    let soft_at = item.point_at(bindings.item_part, &on_item)?;
    let origin = item.weights_at(bindings.item_part, Vec3::ZERO)?;
    let origin_at = item.point_at(bindings.item_part, &origin)?;
    let world = |on_item: [f32; 4], anchor: Vec3, strength: f32| GripLink {
        item: bindings.item_part,
        on_item,
        hand: None,
        on_hand: [0.0; 4],
        slack: 0.0,
        length: 0.0,
        strength,
        smoothing: DEFAULT_SMOOTHING,
        // The original leaves the world links' shares zero; a world anchor
        // does not move, so the item's share is the whole correction.
        flags: RUNTIME | DISABLED,
        shares: (1.0, 0.0),
        anchor: Some(anchor),
    };
    let mut links = Vec::with_capacity(2);
    // The soft weld is only built under the original's auto-parry flag;
    // the stiff origin weld is always built.
    if soft {
        links.push(world(on_item, soft_at, 0.05));
    }
    links.push(world(origin, origin_at, 1.0));
    Some(links)
}

/// The torch's link points on the item, cm: 20 cm below its origin and
/// 10 cm along x.
pub const TORCH_POINTS: [Vec3; 2] = [Vec3::new(0.0, -20.0, 0.0), Vec3::new(10.0, 0.0, 0.0)];

/// Builds a torch's links: its own two pivots (at [`TORCH_POINTS`], both
/// to the holding hand) and its soft world weld at the item's origin
/// (strength 0.05, the world-link flags). A torch is attached by its own
/// routine, not by the grip table, and always gets the soft weld.
/// `holding_hand` is the torch's hand (0 right, 1 left) and `item_to_hand`
/// the torch node's matrix in that hand's frame. `None` when a part or
/// point is malformed.
pub fn torch_links(
    item: &Ragdoll,
    hand: &Ragdoll,
    bindings: GripBindings,
    holding_hand: usize,
    item_to_hand: Affine3A,
) -> Option<GripLinks> {
    let cm = 0.01;
    let holding = *bindings.hand_parts.get(holding_hand)?;
    let mut links = Vec::new();
    for point in TORCH_POINTS {
        let local = point * cm;
        let on_item = item.weights_at(bindings.item_part, local)?;
        let in_hand = item_to_hand.transform_point3(local);
        let on_hand = hand.weights_at(holding, in_hand)?;
        let item_mobility = item.parts.get(bindings.item_part)?.inverse_mass;
        let parent_mobility = hand.link_mobility(holding)?;
        links.push(GripLink {
            item: bindings.item_part,
            on_item,
            hand: Some(holding),
            on_hand,
            slack: 0.0,
            length: 0.0,
            strength: 1.0,
            smoothing: DEFAULT_SMOOTHING,
            flags: PIVOT | RUNTIME,
            shares: shares(item_mobility, parent_mobility),
            anchor: None,
        });
    }
    let main = links.len() - 1;
    let on_item = item.weights_at(bindings.item_part, Vec3::ZERO)?;
    let origin_at = item.point_at(bindings.item_part, &on_item)?;
    let world = links.len();
    links.push(GripLink {
        item: bindings.item_part,
        on_item,
        hand: None,
        on_hand: [0.0; 4],
        slack: 0.0,
        length: 0.0,
        strength: 0.05,
        smoothing: DEFAULT_SMOOTHING,
        flags: RUNTIME | DISABLED,
        shares: (1.0, 0.0),
        anchor: Some(origin_at),
    });
    Some(GripLinks {
        links,
        main,
        second: None,
        arm: None,
        extra: None,
        flip: None,
        flip_point: Vec3::ZERO,
        flip_progress: 0.0,
        flipped: false,
        flip_active: false,
        mode: -1,
        relaxing: false,
        record_dirty: false,
        world_soft: Some(world),
        world_hard: None,
    })
}

/// Builds a shield's links: three runtime pivots from points on
/// the shield — its back at (−20, 0, 0) cm to the forearm of its hand, and
/// its two face points (5, 0, ±15) cm to the left hand, whichever hand holds
/// it — then a soft world link at its origin (strength 0.05, the
/// world-link flags). `holding_hand` is the shield's hand (0 right, 1
/// left), whose forearm is `bindings.arm_parts[holding_hand]`, and
/// `item_to_arm` the shield node's matrix in that forearm's frame. The two
/// face points are brought into the left hand's frame by subtracting
/// `left_hand_offset` (cm) from their forearm-frame place, with no turn, as
/// the original does with the left hand's stored rest translation (Exarust
/// passes the left hand part's origin in its parent). `None` when a part
/// or point is malformed.
pub fn shield_links(
    item: &Ragdoll,
    hand: &Ragdoll,
    bindings: GripBindings,
    holding_hand: usize,
    item_to_arm: Affine3A,
    left_hand_offset: Vec3,
) -> Option<GripLinks> {
    let cm = 0.01;
    let forearm = *bindings.arm_parts.get(holding_hand)?;
    let left = bindings.hand_parts[1];
    let points = [
        Vec3::new(-20.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 15.0),
        Vec3::new(5.0, 0.0, -15.0),
    ];
    let mut links = Vec::new();
    for (i, point) in points.into_iter().enumerate() {
        let local = point * cm;
        let on_item = item.weights_at(bindings.item_part, local)?;
        let in_arm = item_to_arm.transform_point3(local);
        let (parent, on_parent) = if i == 0 {
            (forearm, in_arm)
        } else {
            (left, in_arm - left_hand_offset * cm)
        };
        let on_hand = hand.weights_at(parent, on_parent)?;
        let item_mobility = item.parts.get(bindings.item_part)?.inverse_mass;
        let parent_mobility = hand.link_mobility(parent)?;
        links.push(GripLink {
            item: bindings.item_part,
            on_item,
            hand: Some(parent),
            on_hand,
            slack: 0.0,
            length: 0.0,
            strength: 1.0,
            smoothing: DEFAULT_SMOOTHING,
            flags: PIVOT | RUNTIME,
            shares: shares(item_mobility, parent_mobility),
            anchor: None,
        });
    }
    let on_item = item.weights_at(bindings.item_part, Vec3::ZERO)?;
    let origin_at = item.point_at(bindings.item_part, &on_item)?;
    let world = links.len();
    links.push(GripLink {
        item: bindings.item_part,
        on_item,
        hand: None,
        on_hand: [0.0; 4],
        slack: 0.0,
        length: 0.0,
        strength: 0.05,
        smoothing: DEFAULT_SMOOTHING,
        flags: RUNTIME | DISABLED,
        shares: (1.0, 0.0),
        anchor: Some(origin_at),
    });
    Some(GripLinks {
        links,
        main: 0,
        second: None,
        arm: None,
        extra: None,
        flip: None,
        flip_point: Vec3::ZERO,
        flip_progress: 0.0,
        flipped: false,
        flip_active: false,
        mode: -1,
        relaxing: false,
        record_dirty: false,
        world_soft: Some(world),
        world_hard: None,
    })
}

impl GripLinks {
    /// The grip mode's word changed: the original stores it and, when the
    /// entry has a callback (every kind but a one-hander), marks the links
    /// as still relaxing. Returns whether it changed.
    pub fn set_mode(&mut self, mode: i32, kind: GripKind) -> bool {
        if mode == self.mode {
            return false;
        }
        self.mode = mode;
        if kind != GripKind::Other {
            self.relaxing = true;
        }
        true
    }

    /// Applies a grip mode, the entry's
    /// callback. The two-hander's main link takes a 4 cm slack; modes 0, 1,
    /// 2 and 3 switch the second hand's, arm's and extra links (the extra
    /// with the current distance as its slack), and a polearm's main link
    /// slides with a 15·scale cm slack in modes 1 and 3. `thrust_three` is
    /// the weapon record's thrust class (class 3, which turns a two-handed
    /// mode 2 into 1 for a half-sword grip, `stab_grip`). The callback runs
    /// only when the mode changed, as the mode setter does. Returns false
    /// for a configuration missing a link index (the original reports an
    /// invalid configuration and writes nothing).
    #[allow(clippy::too_many_arguments)]
    pub fn apply_mode(
        &mut self,
        mode: i32,
        kind: GripKind,
        scale: f32,
        thrust_three: bool,
        stab_grip: bool,
        item: &Ragdoll,
        hand: &Ragdoll,
    ) -> bool {
        if mode == self.mode {
            return true;
        }
        let current = |links: &GripLinks, i: usize| -> Option<f32> {
            link_distance(links.links.get(i)?, item, hand)
        };
        // The setter stores the requested mode; the callback's half-sword
        // turn of mode 2 into 1 is local to the callback.
        let requested = mode;
        match kind {
            GripKind::TwoHanded => {
                let (Some(second), Some(arm), Some(extra)) = (self.second, self.arm, self.extra)
                else {
                    return false;
                };
                if self.main >= self.links.len() {
                    return false;
                }
                // The main grip link's slack is 4 cm.
                self.links[self.main].slack = 0.04;
                self.links[self.main].length = 0.04;
                let mode = if mode == 2 && thrust_three && stab_grip {
                    1
                } else {
                    mode
                };
                match mode {
                    0 => {
                        self.links[second].disable();
                        self.links[arm].disable();
                        self.links[extra].disable();
                    }
                    3 => {
                        self.links[second].disable();
                        self.links[arm].disable();
                        if let Some(d) = current(self, extra) {
                            self.links[extra].enable(d);
                        }
                    }
                    2 => {
                        self.links[second].disable();
                        self.links[extra].disable();
                        if let Some(d) = current(self, arm) {
                            self.links[arm].enable(d);
                        }
                    }
                    _ => {
                        self.links[arm].disable();
                        self.links[extra].disable();
                        if let Some(d) = current(self, second) {
                            self.links[second].enable(d);
                        }
                    }
                }
            }
            GripKind::Polearm => {
                let (Some(second), Some(arm)) = (self.second, self.arm) else {
                    return false;
                };
                if self.main >= self.links.len() {
                    return false;
                }
                if mode == 1 || mode == 3 {
                    // The main link slides: no pivot flag, 15·scale cm.
                    self.links[self.main].flags &= !(PIVOT | ENABLED);
                    let slack = 15.0 * scale * 0.01;
                    self.links[self.main].slack = slack;
                    self.links[self.main].length = slack;
                    if let Some(d) = current(self, second) {
                        self.links[second].enable(d);
                    }
                    self.links[arm].disable();
                } else {
                    self.links[second].disable();
                    if let Some(d) = current(self, self.main) {
                        let l = &mut self.links[self.main];
                        l.flags = (l.flags & !DISABLED) | ENABLED;
                        l.slack = d;
                        l.length = d;
                    }
                    if mode == 2 {
                        if let Some(d) = current(self, arm) {
                            self.links[arm].enable(d);
                        }
                    } else {
                        self.links[arm].disable();
                    }
                }
            }
            GripKind::Other => {}
        }
        self.mode = requested;
        if kind != GripKind::Other {
            self.relaxing = true;
        }
        true
    }

    /// One relax pass after a grip change: every link enabled
    /// at least once loses slack — the main link 1 cm, the second hand
    /// `(√s + 0.02)·0.1` cm, the arm `s·0.02 + 0.15` cm and the extra
    /// `(√s + 0.01)·0.4` cm (s in cm) — until it reaches zero, when its
    /// enabled bit clears. `relaxing` stays set while any is left. Returns
    /// false for a missing main or second link (the original's invalid
    /// configuration).
    pub fn relax(&mut self) -> bool {
        let Some(second) = self.second else {
            return false;
        };
        if self.main >= self.links.len() || second >= self.links.len() {
            return false;
        }
        let mut relaxed = false;
        let cm = 100.0;
        fn shrink(link: &mut GripLink, amount_cm: f32, cm: f32) -> bool {
            if !link.once_enabled() {
                return false;
            }
            let left = link.slack * cm - amount_cm;
            if left > 0.0 {
                link.slack = left / cm;
                true
            } else {
                link.slack = 0.0;
                link.flags &= !ENABLED;
                false
            }
        }
        let main = self.main;
        if self.links[main].once_enabled() {
            relaxed |= shrink(&mut self.links[main], 1.0, cm);
        }
        let s = self.links[second].slack * cm;
        if self.links[second].once_enabled() {
            relaxed |= shrink(
                &mut self.links[second],
                (s.max(0.0).sqrt() + 0.02) * 0.1,
                cm,
            );
        }
        if let Some(arm) = self.arm {
            let s = self.links[arm].slack * cm;
            if self.links[arm].once_enabled() {
                relaxed |= shrink(&mut self.links[arm], s * 0.02 + 0.15, cm);
            }
        }
        if let Some(extra) = self.extra {
            let s = self.links[extra].slack * cm;
            if self.links[extra].once_enabled() {
                relaxed |= shrink(&mut self.links[extra], (s.max(0.0).sqrt() + 0.01) * 0.4, cm);
            }
        }
        self.relaxing = relaxed;
        true
    }

    /// The death routine stiffens each held item's soft (parry) world
    /// weld: its strength becomes 1. The original tests the weld's index
    /// as strictly positive, so a weld at index 0 is left alone. Returns
    /// whether a weld was stiffened.
    pub fn weld_on_death(&mut self) -> bool {
        let Some(soft) = self.world_soft.filter(|&i| i > 0 && i < self.links.len()) else {
            return false;
        };
        self.links[soft].strength = 1.0;
        true
    }

    /// Starts a flip if the item has a flip link and its section defines a
    /// flip side (the original's flip command refuses a weapon whose
    /// flipped-side kind byte is zero). The original's caller sets its
    /// "flip active" byte; the direction is the item's current flipped
    /// flag, so a second command turns it back.
    pub fn begin_flip(&mut self, can_flip: bool) -> bool {
        if !can_flip || self.flip.is_none_or(|i| i >= self.links.len()) {
            return false;
        }
        self.flip_active = true;
        true
    }

    /// One flip step while a flip is active: only while the character is
    /// not attacking, or the flip is within 0.49 of its middle. The
    /// progress moves by `(1 − (0.5 − f)²·4)·0.05 + 0.002` a tick toward 1
    /// (or back to 0 once flipped), and at either end the item's record
    /// must be rebuilt and the flip flag turns over. The flip link's point
    /// turns about the item's y axis by `f·π`. Returns whether the link
    /// moved.
    pub fn flip_step(&mut self, item: &Ragdoll, attacking: bool) -> bool {
        if !self.flip_active {
            return false;
        }
        // The middle test subtracts and takes the magnitude in single
        // precision and compares in double; a NaN lets the flip run.
        if attacking && f64::from((self.flip_progress - 0.5).abs()) > 0.49 {
            return false;
        }
        let Some(flip) = self.flip.filter(|&i| i < self.links.len()) else {
            return false;
        };
        // `1 − (0.5 − f)²·4` in single precision, then scaled and offset
        // in double.
        let offset = 0.5 - self.flip_progress;
        let ease = 1.0 - offset * offset * 4.0;
        let step = (f64::from(ease) * 0.05 + 0.002) as f32;
        // The end tests are double compares on the single-precision
        // progress; a NaN keeps the flip running.
        let running = if !self.flipped {
            self.flip_progress += step;
            if f64::from(self.flip_progress).partial_cmp(&0.99) != Some(std::cmp::Ordering::Greater)
            {
                true
            } else {
                self.flip_progress = 1.0;
                self.flipped = true;
                false
            }
        } else {
            self.flip_progress -= step;
            if f64::from(self.flip_progress).partial_cmp(&0.01) != Some(std::cmp::Ordering::Less) {
                true
            } else {
                self.flip_progress = 0.0;
                self.flipped = false;
                false
            }
        };
        if !running {
            self.flip_active = false;
            self.record_dirty = true;
        }
        let angle = (f64::from(self.flip_progress) * std::f64::consts::PI) as f32;
        let (sin, cos) = crate::engine_math::sin_cos(angle);
        let p = self.flip_point * 0.01;
        let turned = Vec3::new(p.x * cos - p.z * sin, p.y, p.z * cos + p.x * sin);
        if let Some(on_item) = item.weights_at(self.links[flip].item, turned) {
            self.links[flip].on_item = on_item;
            true
        } else {
            false
        }
    }
}

/// The grip mode a clip's change asks for: a knocked-down body
/// 0; Idle or Walk 2; Stab 3; a clip outside 0x0E..=0x11, 0x1C and 0x1F
/// while the character's own clip is under 0x2C 1; anything else 0.
pub fn mode_from_clip(clip: i32, current_clip: i32, down: bool) -> i32 {
    if down {
        return 0;
    }
    if clip == 0x13 || clip == 0 {
        2
    } else if clip == 0x0B {
        3
    } else if !(0x0E..=0x11).contains(&clip) && clip != 0x1C && clip != 0x1F && current_clip < 0x2C
    {
        1
    } else {
        0
    }
}

/// A part's weights as the builder needs them (the original's rest frame).
pub fn part_weights(part: &Part, local: Vec3) -> [f32; 4] {
    let edges = bevy::math::Mat3::from_cols(
        part.rest[1] - part.rest[0],
        part.rest[2] - part.rest[0],
        part.rest[3] - part.rest[0],
    );
    let w = edges.inverse() * (local - part.rest[0]);
    [1.0 - w.x - w.y - w.z, w.x, w.y, w.z]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::PartSetup;
    use bevy::math::Affine3A;
    use bevy::prelude::Entity;

    fn body_at(translation: Vec3) -> Ragdoll {
        Ragdoll::new(&[PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(translation),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }])
    }

    fn grip() -> GripSpec {
        GripSpec {
            hand: 0,
            links: vec![
                GripPoint {
                    point: Vec3::new(15.0, 0.0, 0.0),
                    slack: 5.0,
                },
                GripPoint {
                    point: Vec3::new(0.0, -20.0, 0.0),
                    slack: 0.0,
                },
            ],
            second_hand: Some(GripPair {
                item: Vec3::new(0.0, -30.0, 0.0),
                part: Vec3::new(11.0, -3.0, 0.0),
            }),
            arm: Some(GripPair {
                item: Vec3::new(0.0, 20.0, 0.0),
                part: Vec3::new(-12.0, 3.0, 6.0),
            }),
            extra: Some(GripPair {
                item: Vec3::new(0.0, 25.0, 0.0),
                part: Vec3::new(8.0, -5.0, 0.0),
            }),
        }
    }

    fn bindings() -> GripBindings {
        GripBindings {
            item_part: 0,
            hand_parts: [0, 0],
            arm_parts: [0, 0],
        }
    }

    #[test]
    fn a_two_handed_grip_builds_five_links() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let links = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert_eq!(links.links.len(), 5);
        assert_eq!(
            (links.second, links.arm, links.extra),
            (Some(0), Some(1), Some(2))
        );
        assert_eq!(links.main, 4);
        assert_eq!(links.flip, Some(3));
        // The second hand's, the arm's and the extra link start disabled.
        for i in 0..3 {
            assert_eq!(links.links[i].flags, PIVOT | DISABLED);
            assert!(!links.links[i].enabled());
            assert_eq!(links.links[i].shares.0 + links.links[i].shares.1, 1.0);
        }
        // The main links are pivots, the second one with 0.01 smoothing.
        assert_eq!(links.links[3].flags, PIVOT);
        assert!((links.links[3].slack - 0.05).abs() < 1e-6);
        assert_eq!(links.links[3].smoothing, DEFAULT_SMOOTHING);
        assert!(links.links[4].slack.abs() < 1e-9);
        assert_eq!(links.links[4].smoothing, 0.01);
        // The table's points are the links' rest points (scale 1).
        let p = item.rest_point_at(0, &links.links[3].on_item).unwrap();
        assert!((p.x - 0.15).abs() < 1e-6 && p.y.abs() < 1e-6 && p.z.abs() < 1e-6);
        let p = item.rest_point_at(0, &links.links[0].on_item).unwrap();
        assert!((p.y + 0.30).abs() < 1e-6);
        // The hand-side points are the same point mapped by the node matrix.
        let q = hand.rest_point_at(0, &links.links[3].on_hand).unwrap();
        assert!((q.x - 0.15).abs() < 1e-6);
    }

    #[test]
    fn the_scale_and_the_flip_move_the_points() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let mut g = grip();
        g.second_hand = None;
        g.arm = None;
        g.extra = None;
        let links = build(
            &item,
            &hand,
            bindings(),
            &g,
            2.0,
            1.0,
            Affine3A::IDENTITY,
            true,
            1.0,
        )
        .unwrap();
        // Flipped x and z, scaled by 2: (15,0,0) → (−0.30, 0, 0).
        let p = item.rest_point_at(0, &links.links[0].on_item).unwrap();
        assert!((p.x + 0.30).abs() < 1e-6);
        let p = item.rest_point_at(0, &links.links[1].on_item).unwrap();
        assert!((p.y + 0.40).abs() < 1e-6);
        // A right-hand grip's flip link is the second main link.
        assert_eq!(links.flip, Some(1));
        assert_eq!(links.flip_point, Vec3::new(0.0, -40.0, 0.0));
    }

    #[test]
    fn the_two_handed_modes_switch_the_links() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::new(1.0, 0.0, 0.0));
        let mut links = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert!(links.apply_mode(0, GripKind::TwoHanded, 1.0, false, false, &item, &hand));
        assert_eq!(links.mode, 0);
        assert_eq!(links.links[links.main].slack, 0.04);
        assert!(!links.links[0].enabled());
        assert!(links.apply_mode(1, GripKind::TwoHanded, 1.0, false, false, &item, &hand));
        assert!(links.links[0].enabled());
        assert!(links.links[0].once_enabled());
        assert!(links.links[0].slack > 0.0);
        assert!(!links.links[1].enabled());
        assert!(links.apply_mode(2, GripKind::TwoHanded, 1.0, false, false, &item, &hand));
        assert!(links.links[1].enabled());
        assert!(!links.links[0].enabled());
        assert!(links.apply_mode(3, GripKind::TwoHanded, 1.0, false, false, &item, &hand));
        assert!(links.links[2].enabled());
        assert!(!links.links[0].enabled());
        // A half-sword thrust turns mode 2 into 1 inside the callback; the
        // stored mode stays the requested 2, so asking again does nothing.
        assert!(links.apply_mode(2, GripKind::TwoHanded, 1.0, true, true, &item, &hand));
        assert_eq!(links.mode, 2);
        assert!(links.links[0].enabled());
        links.links[0].disable();
        assert!(links.apply_mode(2, GripKind::TwoHanded, 1.0, true, true, &item, &hand));
        assert!(!links.links[0].enabled());
    }

    #[test]
    fn a_polearm_slides_in_its_swing_modes() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let mut g = grip();
        g.extra = None;
        let mut links = build(
            &item,
            &hand,
            bindings(),
            &g,
            2.0,
            1.0,
            Affine3A::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            false,
            1.0,
        )
        .unwrap();
        // Modes 1 and 3 make the main link slide (15·scale cm).
        assert!(links.apply_mode(1, GripKind::Polearm, 2.0, false, false, &item, &hand));
        assert_eq!(links.links[links.main].flags & PIVOT, 0);
        assert!((links.links[links.main].slack - 0.30).abs() < 1e-6);
        assert!(links.links[0].enabled());
        assert!(!links.links[1].enabled());
        // Mode 2 brings the arm in and the main link back to a pivot.
        assert!(links.apply_mode(2, GripKind::Polearm, 2.0, false, false, &item, &hand));
        assert!(links.links[1].enabled());
        assert!(!links.links[0].enabled());
        assert!(links.links[links.main].enabled());
        assert!(links.links[links.main].slack > 0.0);
    }

    #[test]
    fn a_relax_pass_takes_slack_off_the_enabled_links() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::new(1.0, 0.0, 0.0));
        let mut links = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        links.apply_mode(1, GripKind::TwoHanded, 1.0, false, false, &item, &hand);
        // The two-hander's main link is a pivot and never carries the
        // enabled bit, so only the second hand's link relaxes.
        let before = links.links[0].slack;
        let main_before = links.links[links.main].slack;
        assert!(links.relax());
        assert!(links.links[0].slack < before);
        assert_eq!(links.links[links.main].slack, main_before);
        // A missing second link is the original's invalid configuration.
        let mut g = grip();
        g.second_hand = None;
        g.arm = None;
        g.extra = None;
        let mut one = build(
            &item,
            &hand,
            bindings(),
            &g,
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert!(!one.relax());
    }

    #[test]
    fn the_flip_turns_the_point_about_y_and_finishes() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let mut links = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert!(links.begin_flip(true));
        for _ in 0..200 {
            if !links.flip_active {
                break;
            }
            links.flip_step(&item, false);
        }
        assert_eq!(links.flip_progress, 1.0);
        assert!(links.flipped);
        assert!(!links.flip_active);
        assert!(links.record_dirty);
        // At f = 1 the flip point (15, 0, 0) has turned to (−15, 0, 0).
        let p = item
            .rest_point_at(0, &links.links[links.flip.unwrap()].on_item)
            .unwrap();
        assert!((p.x + 0.15).abs() < 1e-5 && p.y.abs() < 1e-5);
        // An attack away from the middle refuses the step.
        let mut links2 = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert!(!links2.flip_active);
        assert!(!links2.flip_step(&item, true));
        assert!(links2.begin_flip(true));
        assert!(!links2.flip_step(&item, true));
        // Near the middle an attack lets the flip run.
        links2.flip_progress = 0.5;
        assert!(links2.flip_step(&item, true));
        // The middle test is a double compare: 0.99 in single precision is
        // 0.490000009… from the middle, past the double 0.49, so an attack
        // holds it (a single-precision compare would let it run).
        links2.flip_progress = 0.99;
        assert!(!links2.flip_step(&item, true));
        // From 0.25 the step is (1 − 0.25²·4)·0.05 + 0.002 = 0.0395, worked
        // in double and stored as a single.
        links2.flip_progress = 0.25;
        assert!(links2.flip_step(&item, false));
        assert_eq!(links2.flip_progress, 0.25 + 0.0395_f64 as f32);
    }

    #[test]
    fn the_world_links_and_the_mode_word() {
        let item = body_at(Vec3::ZERO);
        let one = world_links(&item, bindings(), Vec3::new(0.0, 10.0, 0.0), false).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].strength, 1.0);
        assert_eq!(one[0].shares, (1.0, 0.0));
        let two = world_links(&item, bindings(), Vec3::new(0.0, 10.0, 0.0), true).unwrap();
        let (soft, hard) = (&two[0], &two[1]);
        assert_eq!(soft.strength, 0.05);
        assert_eq!(hard.strength, 1.0);
        assert_eq!(soft.flags, RUNTIME | DISABLED);
        assert_eq!(hard.shares, (1.0, 0.0));
        let p = item.rest_point_at(0, &soft.on_item).unwrap();
        assert!((p - Vec3::new(0.0, 0.1, 0.0)).length() < 1e-6);
        let mut links = GripLinks {
            links: Vec::new(),
            main: 0,
            second: None,
            arm: None,
            extra: None,
            flip: None,
            flip_point: Vec3::ZERO,
            flip_progress: 0.0,
            flipped: false,
            flip_active: false,
            mode: -1,
            relaxing: false,
            record_dirty: false,
            world_soft: None,
            world_hard: None,
        };
        assert!(links.set_mode(1, GripKind::TwoHanded));
        assert!(!links.set_mode(1, GripKind::TwoHanded));
        assert!(links.relaxing);
        // A one-hander has no callback, so it marks nothing to relax.
        links.relaxing = false;
        assert!(links.set_mode(4, GripKind::Other));
        assert!(!links.relaxing);
    }

    #[test]
    fn death_stiffens_the_shields_world_weld() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let mut shield =
            shield_links(&item, &hand, bindings(), 1, Affine3A::IDENTITY, Vec3::ZERO).unwrap();
        let soft = shield.world_soft.unwrap();
        assert_eq!(shield.links[soft].strength, 0.05);
        assert!(shield.weld_on_death());
        assert_eq!(shield.links[soft].strength, 1.0);
        // A weapon's stiff origin weld is not the shield's soft index.
        let mut one = GripLinks {
            links: Vec::new(),
            main: 0,
            second: None,
            arm: None,
            extra: None,
            flip: None,
            flip_point: Vec3::ZERO,
            flip_progress: 0.0,
            flipped: false,
            flip_active: false,
            mode: -1,
            relaxing: false,
            record_dirty: false,
            world_soft: None,
            world_hard: Some(0),
        };
        assert!(!one.weld_on_death());
    }

    #[test]
    fn a_shields_three_links_and_its_world_link() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let links =
            shield_links(&item, &hand, bindings(), 1, Affine3A::IDENTITY, Vec3::ZERO).unwrap();
        assert_eq!(links.links.len(), 4);
        assert_eq!(links.world_soft, Some(3));
        for (i, point) in [(-0.20, 0.0, 0.0), (0.05, 0.0, 0.15), (0.05, 0.0, -0.15)]
            .into_iter()
            .enumerate()
        {
            assert_eq!(links.links[i].flags, PIVOT | RUNTIME);
            assert_eq!(links.links[i].strength, 1.0);
            let p = item.rest_point_at(0, &links.links[i].on_item).unwrap();
            assert!((p - Vec3::new(point.0, point.1, point.2)).length() < 1e-6);
        }
        let world = &links.links[3];
        assert_eq!(world.strength, 0.05);
        assert_eq!(world.flags, RUNTIME | DISABLED);
        assert_eq!(world.hand, None);
        assert!(item.rest_point_at(0, &world.on_item).unwrap().length() < 1e-6);
    }

    #[test]
    fn the_mode_from_the_clip() {
        assert_eq!(mode_from_clip(0x13, 0, false), 2);
        assert_eq!(mode_from_clip(0, 0, false), 2);
        assert_eq!(mode_from_clip(0x0B, 0, false), 3);
        assert_eq!(mode_from_clip(5, 0, false), 1);
        assert_eq!(mode_from_clip(0x0E, 0, false), 0);
        assert_eq!(mode_from_clip(0x11, 0, false), 0);
        assert_eq!(mode_from_clip(0x1C, 0, false), 0);
        assert_eq!(mode_from_clip(0x1F, 0, false), 0);
        assert_eq!(mode_from_clip(5, 0x2C, false), 0);
        assert_eq!(mode_from_clip(5, 0, true), 0);
    }

    #[test]
    fn a_torch_holds_by_its_own_two_links_and_its_soft_weld() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::new(0.3, 0.0, 0.0));
        let links = torch_links(&item, &hand, bindings(), 0, Affine3A::IDENTITY).unwrap();
        assert_eq!(links.links.len(), 3);
        assert_eq!(links.main, 1);
        assert_eq!(links.world_soft, Some(2));
        assert_eq!(links.world_hard, None);
        assert!(links.flip.is_none());
        for (i, point) in TORCH_POINTS.into_iter().enumerate() {
            let p = item.rest_point_at(0, &links.links[i].on_item).unwrap();
            assert!((p - point * 0.01).length() < 1e-6, "{i}: {p}");
            assert_eq!(links.links[i].flags, PIVOT | RUNTIME);
            assert_eq!(links.links[i].strength, 1.0);
            assert_eq!(links.links[i].hand, Some(0));
        }
        let world = &links.links[2];
        assert_eq!(world.strength, 0.05);
        assert_eq!(world.flags, RUNTIME | DISABLED);
        let p = item.rest_point_at(0, &world.on_item).unwrap();
        assert!(p.length() < 1e-6);

        // The torch hangs from its own hand: a left-hand torch takes the
        // second hand part (here one the holder lacks, so nothing builds).
        let left_missing = GripBindings {
            hand_parts: [0, 5],
            ..bindings()
        };
        assert!(torch_links(&item, &hand, left_missing, 0, Affine3A::IDENTITY).is_some());
        assert!(torch_links(&item, &hand, left_missing, 1, Affine3A::IDENTITY).is_none());
    }

    #[test]
    fn the_flip_command_needs_a_flip_side() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let mut links = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert!(!links.begin_flip(false));
        assert!(!links.flip_active);
        assert!(links.begin_flip(true));
        assert!(links.flip_active);
        // A shield has no flip link: the command refuses it.
        let mut shield =
            shield_links(&item, &hand, bindings(), 1, Affine3A::IDENTITY, Vec3::ZERO).unwrap();
        assert!(shield.flip.is_none());
        assert!(!shield.begin_flip(true));
        assert!(!shield.flip_active);
    }

    #[test]
    fn a_death_stiffens_only_a_soft_weld() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let mut shield =
            shield_links(&item, &hand, bindings(), 1, Affine3A::IDENTITY, Vec3::ZERO).unwrap();
        let soft = shield.world_soft.unwrap();
        assert!(shield.weld_on_death());
        assert_eq!(shield.links[soft].strength, 1.0);
        // A group with no soft weld has nothing to stiffen.
        let mut weapon = build(
            &item,
            &hand,
            bindings(),
            &grip(),
            1.0,
            1.0,
            Affine3A::IDENTITY,
            false,
            1.0,
        )
        .unwrap();
        assert!(!weapon.weld_on_death());
    }

    #[test]
    fn a_shields_face_points_take_the_left_hands_offset() {
        let item = body_at(Vec3::ZERO);
        let hand = body_at(Vec3::ZERO);
        let none =
            shield_links(&item, &hand, bindings(), 1, Affine3A::IDENTITY, Vec3::ZERO).unwrap();
        // The offset is subtracted after the item node's turn, cm.
        let shifted = shield_links(
            &item,
            &hand,
            bindings(),
            1,
            Affine3A::IDENTITY,
            Vec3::new(5.0, 1.0, 0.0),
        )
        .unwrap();
        for i in 0..3 {
            let a = hand.rest_point_at(0, &none.links[i].on_hand).unwrap();
            let b = hand.rest_point_at(0, &shifted.links[i].on_hand).unwrap();
            let want = if i == 0 {
                Vec3::ZERO
            } else {
                Vec3::new(-0.05, -0.01, 0.0)
            };
            assert!((b - a - want).length() < 1e-6, "{i}: {:?}", b - a);
        }
        // The back point goes to the holding hand's forearm and the face
        // points to the left hand, whichever hand holds the shield (part 5
        // is one the holder lacks, so a link to it refuses the build).
        let parts = |hand_parts, arm_parts| GripBindings {
            item_part: 0,
            hand_parts,
            arm_parts,
        };
        let build = |b, holding| {
            shield_links(&item, &hand, b, holding, Affine3A::IDENTITY, Vec3::ZERO).is_some()
        };
        assert!(build(parts([0, 0], [0, 5]), 0));
        assert!(!build(parts([0, 0], [0, 5]), 1));
        assert!(!build(parts([0, 5], [0, 0]), 0));
        assert!(build(parts([5, 0], [0, 0]), 0));
    }
}

#[cfg(test)]
mod empty_hand_tests {
    use super::*;

    /// Emptying the hands restores a 0.175 share of the base mass as the
    /// link mobility and the base mass itself.
    #[test]
    fn an_empty_hand_takes_its_own_weights_back() {
        assert_eq!(empty_hand_words(2.0), (0.35, 2.0));
        assert_eq!(empty_hand_words(1.0), (0.175, 1.0));
        assert_eq!(empty_hand_words(0.0), (0.0, 0.0));
        // Unarmed, the equip pass halves the mass word (a quarter with
        // heavy claws).
        assert_eq!(unarmed_hand_mobility(2.0, false), 1.0);
        assert_eq!(unarmed_hand_mobility(2.0, true), 0.5);
        assert_eq!(unarmed_hand_mobility(0.35, false), 0.175);
    }
}
