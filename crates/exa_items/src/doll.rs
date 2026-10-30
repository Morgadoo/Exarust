//! The character doll's equipment pipeline: what a view attaches to a
//! figure's hands, how an item is taken into them, how one is taken out,
//! and what the record keeps of it.
//!
//! A plan compares the record's stowed set with its out set, hand by hand
//! (main against main, off against off). Only a hand whose item differs is
//! re-dressed: its old model is detached, then the stowed item is attached
//! by its sections — a weapon as a weapon part (an off-hand item that also
//! has a light section is attached as an off-hand light instead), a shield
//! as a shield, any other item with a light section as an off-hand light,
//! and anything else is not attached at all. A hand's `0x1000000` flag
//! flips the item and its `0x2000000` flag lights it. When neither hand
//! changes there is no plan, the sets do not trade, and the record keeps
//! its out set.
//!
//! Taking an item into the hands follows the branch order of
//! [`crate::possession::hold_plan`]: a one-handed light beside a one-handed
//! weapon or the fists frees the off hand and goes there; a light already
//! in the main hand moves to the off hand (or the off hand is freed); a
//! second weapon goes to the off hand only when asked for, never beside a
//! two-hander; everything else frees the main hand. A two-hander frees the
//! off hand and then the main hand. The item is then attached as a weapon
//! (or as the off-hand light) and its record entry written.
//!
//! Taking a weapon out of a hand clears the record's entry that names it
//! (either hand's), and — when the take-out was scheduled, it was the main
//! hand and the weapon was the first of the body's attached weapons —
//! promotes the off hand's item into the main hand if that item is a light
//! or a weapon (a shield is not promoted). The promoted item takes the off
//! hand's flags, as the original copies them. Taking out the off hand's
//! light or shield reads and clears the record's off-hand entry whatever
//! item it names.
//!
//! Applying a plan clears the body's weapon part between the detaches and
//! the attaches when the body's weapon class word has no `0x30` bits, and
//! refreshes the body after the attaches when both hands of the stowed set
//! the plan read are empty.
//!
//! Every completed re-dress recomputes the body's grip (its weapon class
//! word from the parts it holds), restarts the figure's stance with blend
//! 20 unless the body is in its weapon-swap state (its animation does it
//! then), and clears the body's first attach flag.
//!
//! The doll's hand scene is built from the record's other set: each hand
//! whose item differs gets a model when the item has one, attached as a
//! weapon (the off hand's weapon that also lights as that light), a shield
//! or a light; a modelled item with none of those sections is attached as a
//! weapon. A weapon hand node settles at -1, a shield's at the largest
//! word. Taking an item into a hand writes only the top bit of the dragged
//! item's flags into the record (a shield keeps all of them), while a
//! burning item's lit mark and its flipped mark are set on the model from
//! the full flags. Releasing a hand's old model detaches its scene node,
//! refreshes the body when neither record hand names an item, and restarts
//! the stance when the take-out job's mode has bit 1. A set swap writes the
//! view's two `0x60`-frame counters onto the side the second-set flag
//! leaves.
//!
//! Evidence: the apply, plan, take-out, take-in, shield and swap routines
//! of the character view's equipment cluster, read in the instructions
//! (`context/re/item-gui-lane.md`).

use crate::possession::{
    Attach, FLIPPED, Hand, HoldInto, HoldPlan, HoldStep, ItemKind, LIT, Possessions, hold_plan,
};

/// The blend a re-dress restarts the stance with.
pub const STANCE_BLEND: u32 = 0x14;
/// The body state whose animation swaps the weapon sets; the apply step
/// does not restart the stance then.
pub const SWAP_STATE: u32 = 0x1F;
/// The take-out job's mode bit that restarts the stance (and refreshes the
/// textures) when the hand's old model is released.
pub const RESTART_MODE: u32 = 0x2;
/// The only flag bits an item taken into a hand keeps in the record: the
/// weapon's and the off-hand light's record entry is `flags &` this.
pub const KEPT_HAND_FLAGS: u32 = 0x8000_0000;
/// The node flag a detached hand model takes.
pub const DETACHED_NODE: u32 = 0x800;
/// A weapon hand node's settle word (its item counts as settled).
pub const WEAPON_SETTLE: i32 = -1;
/// A shield hand node's settle word.
pub const SHIELD_SETTLE: i32 = 0x7FFF_FFFF;
/// The view counter a set swap sets: this many frames on the side the
/// second-set flag leaves.
pub const SWAP_TIME: i32 = 0x60;

/// How an item is attached to a hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachKind {
    /// A weapon, in the weapon part of this hand.
    Weapon,
    /// A shield, in the off hand.
    Shield,
    /// A burning item, as the off hand's light.
    Light,
}

/// What a hand of a plan takes, when it changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandAttach {
    pub id: u32,
    /// The item's record flags: a weapon's flip and lit bits are read from
    /// these.
    pub flags: u32,
    pub kind: AttachKind,
}

/// The plan that re-dresses the hands from the record's stowed set: which
/// hands change and what each takes. A hand that changes to an item with
/// no attachable section has no [`HandAttach`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachPlan {
    pub changed: [bool; 2],
    pub attach: [Option<HandAttach>; 2],
    /// The record's stowed set's item ids as the plan read them; the
    /// rebuild refreshes the body when both are 0.
    pub targets: [u32; 2],
    /// The record's second set is out after this plan (the plan was made
    /// for the swap, so the sets trade when it is applied).
    pub swap: bool,
}

impl AttachPlan {
    /// Whether either hand changes; a plan with neither is not made.
    pub fn any(&self) -> bool {
        self.changed[0] || self.changed[1]
    }

    /// Whether the rebuild refreshes the body: both target item ids the
    /// plan read are 0 (the job's own copies of the record's other set).
    pub fn refresh_body(&self) -> bool {
        self.targets == [0, 0]
    }

    /// The scene rebuild's steps in the original's order: each changed hand
    /// whose body model is present detaches it, the body's weapon parts are
    /// cleared when the class word has no `0x30` bits, the new models are
    /// attached in hand order, the body is refreshed when both targets are
    /// 0, the body's grip is recomputed, the stance is restarted unless the
    /// body is in [`SWAP_STATE`], the body's first attach flag is cleared,
    /// and a swap plan trades the record's sets.
    /// `current` are the item ids the body's hand models hold now.
    pub fn rebuild_steps(
        &self,
        current: &[Option<u32>; 2],
        class: u32,
        body_state: u32,
    ) -> Vec<ApplyStep> {
        let mut steps = Vec::new();
        for (hand, changed) in self.changed.iter().enumerate() {
            if *changed && current[hand].is_some() {
                steps.push(ApplyStep::Detach(hand));
            }
        }
        if class & 0x30 == 0 {
            steps.push(ApplyStep::ClearWeaponPart);
        }
        for hand in 0..2 {
            if let Some(attach) = self.attach[hand] {
                steps.push(ApplyStep::Attach {
                    hand,
                    kind: attach.kind,
                });
            }
        }
        if self.refresh_body() {
            steps.push(ApplyStep::RefreshBody);
        }
        steps.push(ApplyStep::RefreshGrip);
        if body_state != SWAP_STATE {
            steps.push(ApplyStep::RestartStance);
        }
        steps.push(ApplyStep::ClearAttachFlag);
        steps
    }
}

/// The plan that turns the hands holding `out` into the record's `stowed`
/// set, as the original builds it before a swap: per hand, changed when the
/// stowed hand's item differs from the out hand's; a weapon section makes
/// it a weapon (the off hand's weapon that also has a light section an
/// off-hand light), else a shield section a shield, else a light section an
/// off-hand light, and anything else is left unattached. `None` when
/// neither hand changes — the original frees the job then, and the record's
/// sets do not trade.
pub fn plan_attachments<F>(out: &[Hand; 2], stowed: &[Hand; 2], kind: F) -> Option<AttachPlan>
where
    F: Fn(u32) -> Option<ItemKind>,
{
    let mut plan = AttachPlan {
        changed: [false; 2],
        attach: [None; 2],
        targets: [stowed[0].id, stowed[1].id],
        swap: true,
    };
    for hand in 0..2 {
        plan.changed[hand] = stowed[hand].id != out[hand].id;
        if !plan.changed[hand] || stowed[hand].id == 0 {
            continue;
        }
        let Some(k) = kind(stowed[hand].id) else {
            continue;
        };
        let Some(attach) = attach_of(k, hand) else {
            continue;
        };
        plan.attach[hand] = Some(HandAttach {
            id: stowed[hand].id,
            flags: stowed[hand].flags,
            kind: attach,
        });
    }
    plan.any().then_some(plan)
}

/// The plan for the record's hands as they are now, as the redress and the
/// initial dressing make it: the hands are compared with the record's out
/// set and the ones that differ are attached from it. No swap happens.
pub fn plan_from_record<F>(hands: &[Hand; 2], record: &[Hand; 2], kind: F) -> Option<AttachPlan>
where
    F: Fn(u32) -> Option<ItemKind>,
{
    plan_attachments(hands, record, kind).map(|plan| AttachPlan {
        swap: false,
        ..plan
    })
}

/// One step of applying a plan, in the original's order: the changed hands
/// detach first (main, then off), the body's weapon part is cleared when
/// its class word has no weapon bits, then the new items attach (main, then
/// off).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyStep {
    Detach(usize),
    /// The body's weapon part is cleared: its weapon class word has none
    /// of the `0x30` bits. Once per apply, whatever the hands take.
    ClearWeaponPart,
    Attach {
        hand: usize,
        kind: AttachKind,
    },
    /// The body is refreshed because both of the plan's target item ids
    /// are 0.
    RefreshBody,
    /// The body's grip is recomputed from the parts it holds (its weapon
    /// class word and hold kind), always.
    RefreshGrip,
    /// The stance is restarted with blend [`STANCE_BLEND`] unless the body
    /// is in the swap state [`SWAP_STATE`].
    RestartStance,
    /// The body's first attach flag is cleared.
    ClearAttachFlag,
    /// The record's sets trade places.
    ExchangeSets,
}

/// Applies an attachment plan to the record: the scene rebuild's steps
/// ([`AttachPlan::rebuild_steps`]) with `current` the item ids the body's
/// hand models hold now, and — for a swap plan — the out and stowed sets
/// exchanged. Returns the steps in the original's order.
pub fn apply(
    p: &mut Possessions,
    plan: &AttachPlan,
    current: &[Option<u32>; 2],
    body_state: u32,
    class: u32,
) -> Vec<ApplyStep> {
    let mut steps = plan.rebuild_steps(current, class, body_state);
    if plan.swap {
        p.exchange_sets();
        steps.push(ApplyStep::ExchangeSets);
    }
    steps
}

/// Starts a weapon-set swap: the second-set flag toggles, the held marks
/// move to the set that becomes the out set ([`Possessions::swap_plan`]),
/// and the plan for the hands is made. The sets themselves trade only in
/// [`Possessions::exchange_sets`], one apply step later.
pub fn begin_swap<F>(p: &mut Possessions, kind: F) -> Option<AttachPlan>
where
    F: Fn(u32) -> Option<ItemKind>,
{
    let _ = p.swap_plan(&kind);
    plan_attachments(&p.out, &p.stowed, kind)
}

/// The equipment job a view has in flight: a swap asked for while one is
/// pending does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Busy(pub bool);

impl Busy {
    /// The swap request gate: refused while a job is in flight.
    pub fn request(&self) -> bool {
        !self.0
    }
}

/// What a take-out job asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TakeOut {
    /// The hand it was made for.
    pub hand: usize,
    /// The job mode word: bit 1 ([`RESTART_MODE`]) restarts the stance and
    /// refreshes the textures when the old model is released, bit 2 asks
    /// for the light finaliser (apparel rebuilt only) on an off-hand light
    /// or shield.
    pub mode: u32,
    /// The item the take-out makes room for: the hand's model must not
    /// already hold it (a second job for the same hand does nothing). `None`
    /// is the null the scheduled drag-off job carries, which always
    /// proceeds.
    pub instead: Option<u32>,
    /// The record's entry is cleared.
    pub record: bool,
    /// The job was scheduled by a drag off the figure: the off hand's
    /// weapon or light is promoted into the main hand when this was the
    /// main hand.
    pub scheduled: bool,
}

impl TakeOut {
    /// The take-out a drag off the figure schedules for `hand`: mode 7, no
    /// expected item, the record cleared and the promotion armed.
    pub fn scheduled(hand: usize) -> Self {
        Self {
            hand,
            mode: 7,
            instead: None,
            record: true,
            scheduled: true,
        }
    }

    /// A plain take-out that makes room for another item in `hand` and
    /// keeps the record.
    pub fn plain(hand: usize, instead: u32) -> Self {
        Self {
            hand,
            mode: 0,
            instead: Some(instead),
            record: false,
            scheduled: false,
        }
    }
}

/// Where the item a take-out removes sits on the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldPlace {
    /// One of the body's attached weapons; `first` when it is the first of
    /// them (the promotion reads it).
    Weapon { first: bool },
    /// The off hand's light.
    Light,
    /// The off hand's shield.
    Shield,
}

/// What a take-out did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Released {
    /// The item left the hand.
    pub done: bool,
    /// The flags of the record entry that named it (0 when none did).
    pub flags: u32,
    /// The off hand's item moved into the main hand.
    pub promoted: Option<u32>,
    /// The stance restarts and the textures refresh (mode bit 1).
    pub restarts: bool,
    /// Neither out hand holds an item after the take-out.
    pub both_empty: bool,
    /// The light finaliser runs ([`FinishKind::Light`]): an off-hand light
    /// was taken out by a job whose mode has bit 2.
    pub light_finish: bool,
}

/// Takes an item out of a hand, as a drag off the figure, a take-in and a
/// shield take do. `held` is the item in the hand's model and where it sits
/// (`None` when the hand is empty), `off` the off hand's model's item and
/// kind as it is once the item has left.
///
/// The hand must be one of the two, must hold a model, and that model must
/// not be the item the take-out makes room for (a hand that already holds
/// it is left alone). A weapon's record entry is the one naming it, in
/// either hand; it is cleared when `record` is set. With `scheduled`, the
/// main hand and the body's first weapon, the off hand's item is promoted
/// into the main hand — it takes the off hand's flags — when it is a light
/// or a weapon; a shield stays in the off hand. An off-hand light or shield
/// reads the record's off-hand flags and clears that entry without
/// comparing its item.
pub fn take_out(
    p: &mut Possessions,
    job: &TakeOut,
    held: Option<(u32, HeldPlace)>,
    off: Option<(u32, ItemKind)>,
) -> Released {
    let mut released = Released {
        restarts: job.mode & RESTART_MODE != 0,
        ..Default::default()
    };
    let hand = job.hand;
    if hand > 1 {
        return released;
    }
    let Some((held, place)) = held else {
        return released;
    };
    if job.instead == Some(held) {
        return released;
    }
    released.done = true;
    match place {
        HeldPlace::Weapon { first } => {
            if p.out[hand].id == held {
                released.flags = p.out[hand].flags;
                if job.record {
                    p.out[hand].id = 0;
                }
            } else if p.out[1 - hand].id == held {
                released.flags = p.out[1 - hand].flags;
                if job.record {
                    p.out[1 - hand].id = 0;
                }
            }
            if job.scheduled
                && first
                && hand == 0
                && let Some((off_id, off_kind)) = off
                && (off_kind.light || off_kind.weapon.is_some())
            {
                if job.record {
                    p.out[0] = Hand {
                        id: off_id,
                        flags: p.out[1].flags,
                    };
                    p.out[1].id = 0;
                }
                released.promoted = Some(off_id);
            }
        }
        HeldPlace::Light | HeldPlace::Shield => {
            released.flags = p.out[1].flags;
            if job.record {
                p.out[1].id = 0;
            }
            released.light_finish = place == HeldPlace::Light && job.mode & 4 != 0;
        }
    }
    released.both_empty = p.out[0].id == 0 && p.out[1].id == 0;
    released
}

/// What taking an item into the hands does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Into {
    /// The removals first, then the hand it goes into.
    pub plan: HoldPlan,
    /// The item is attached as a weapon, or as the off-hand light.
    pub kind: AttachKind,
    /// The record entry the take-in writes: the item's flags less all but
    /// [`KEPT_HAND_FLAGS`], as the weapon and light attach jobs keep them.
    pub flags: u32,
    /// The light that moved from the main hand to the off hand: the
    /// record's off hand takes the light's item and the *new* item's
    /// flags, as the original copies the job's flags.
    pub moved_light: Option<Hand>,
}

/// The facts a take-in needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntoFacts {
    pub kind: ItemKind,
    /// The main hand's item and whether it is a light.
    pub main: Option<(u32, bool)>,
    pub off: Option<u32>,
    /// The body's weapon class word.
    pub class: u32,
    /// The off hand was asked for.
    pub off_wanted: bool,
}

/// Takes an item into the hands: the item's take-in plan decides the
/// removals and the hand, and the item is attached as a weapon or as the
/// off-hand light. `flags` are the item's record flags; the weapon and
/// light attach jobs keep only [`KEPT_HAND_FLAGS`] of them, and a light
/// moving out of the main hand takes that record's flags with it.
///
/// The original's job first waits, run after run, until the item's scene
/// node and its part report themselves loaded; Exarust's items are whole
/// when the job is made, so there is nothing to wait for.
pub fn take_into_hands(facts: IntoFacts, flags: u32) -> Into {
    let flags = flags & KEPT_HAND_FLAGS;
    let plan = hold_plan(
        facts.kind.grip(),
        facts.kind.light,
        facts
            .main
            .map(|(id, light)| crate::possession::Holding { id, light }),
        facts
            .off
            .map(|id| crate::possession::Holding { id, light: false }),
        facts.class,
        facts.off_wanted,
    );
    let mut moved_light = None;
    if let Some((light, _)) = facts.main
        && plan.steps.contains(&HoldStep::LightToOff)
    {
        moved_light = Some(Hand { id: light, flags });
    }
    let attach = match plan.into {
        HoldInto::OffLight => AttachKind::Light,
        HoldInto::Hand(_) => AttachKind::Weapon,
    };
    Into {
        plan,
        kind: attach,
        flags,
        moved_light,
    }
}

/// Records an item taken into a hand: the hand's entry becomes the item
/// and its flags. The off-hand light always writes the off hand, whichever
/// hand the take-in reported.
pub fn record_into(p: &mut Possessions, plan: &HoldPlan, item: u32, flags: u32) {
    let hand = match plan.into {
        HoldInto::Hand(hand) => hand,
        HoldInto::OffLight => 1,
    };
    p.record_hold(hand, item, flags);
}

/// What taking a shield into the off hand did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldTaken {
    /// The off hand's old item is released first.
    pub frees_off: bool,
    /// The body's weapon part is cleared (it holds no weapon class bits).
    pub clears_part: bool,
}

/// Takes a shield into the off hand: the off hand is released first (a
/// take-out of the off hand that makes room for the shield and clears the
/// record), the record's off hand takes the shield and its flags, and the
/// body's weapon part is cleared when it holds no weapon class bits. `off`
/// is the item in the off hand's model and where it sits.
pub fn take_shield(
    off: Option<(u32, HeldPlace)>,
    item: u32,
    flags: u32,
    p: &mut Possessions,
    class: u32,
) -> ShieldTaken {
    if off.is_some() {
        let job = TakeOut {
            record: true,
            ..TakeOut::plain(1, item)
        };
        take_out(p, &job, off, None);
    }
    p.record_hold(1, item, flags);
    ShieldTaken {
        frees_off: off.is_some(),
        clears_part: class & 0x30 == 0,
    }
}

/// The attach a hand takes, for callers that only have the item's kind.
pub fn attach_of(kind: ItemKind, hand: usize) -> Option<AttachKind> {
    if kind.weapon.is_some() {
        if hand == 1 && kind.light {
            Some(AttachKind::Light)
        } else {
            Some(AttachKind::Weapon)
        }
    } else if kind.shield {
        Some(AttachKind::Shield)
    } else if kind.light {
        Some(AttachKind::Light)
    } else {
        None
    }
}

/// The dress the hands from the record, as the loadout and trade screens
/// do: the record's out set is replaced by the items the hand models hold
/// (ids only), a two-handed item (both models the same) fills the main hand
/// alone, and the plan attaches the record's real out set over them. The
/// record is not changed. `None` when no hand differs.
pub fn redress<F>(p: &Possessions, held: [Option<u32>; 2], kind: F) -> Option<AttachPlan>
where
    F: Fn(u32) -> Option<ItemKind>,
{
    let mut now = [Hand::default(); 2];
    for hand in 0..2 {
        now[hand].id = held[hand].unwrap_or(0);
        if hand == 0 && now[0].id != 0 && now[1].id == now[0].id {
            now[1].id = 0;
        }
    }
    plan_from_record(&now, &p.out, kind)
}

/// The marks an attached item's model takes from its record flags: it is
/// held flipped when `0x1000000` is set (the body keeps the bit) and its
/// held part burns when `0x2000000` is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModelMarks {
    pub flipped: bool,
    pub lit: bool,
}

/// The marks for an item's record flags.
pub fn model_marks(flags: u32) -> ModelMarks {
    ModelMarks {
        flipped: flags & FLIPPED != 0,
        lit: flags & LIT != 0,
    }
}

/// The job a model attach leaves for the next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttachJob {
    /// A weapon hands over to the take-in job, which lights the part,
    /// aims the swing, restarts the stance and runs a finaliser.
    #[default]
    TakeIn,
    /// A shield hands over to the node finaliser, which takes the node
    /// out and refreshes the body.
    NodeFinal,
}

/// What taking a weapon's model into a hand does before the take-in job
/// runs: an item flagged `0x2000000` lights its held part, an item whose
/// object record answers the `0x100` test has every mesh of its held model
/// marked and every mesh part set to cast shadows, a body with no weapon
/// class bits has its weapon part cleared first, and the job is the
/// take-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeaponAttach {
    pub lit: bool,
    /// The held model's meshes cast shadows (the object record's `0x100`
    /// test).
    pub casts_shadow: bool,
    pub clears_part: bool,
    pub job: AttachJob,
}

/// The weapon model's marks, the body's part reset and its job.
/// `shadow_mark` is the object record's `0x100` test for the item.
pub fn weapon_attach(flags: u32, class: u32, shadow_mark: bool) -> WeaponAttach {
    WeaponAttach {
        lit: flags & LIT != 0,
        casts_shadow: shadow_mark,
        clears_part: class & 0x30 == 0,
        job: AttachJob::TakeIn,
    }
}

/// What a shield's model does: the body's weapon part is cleared the same
/// way (and its part is marked `0x800`, not modelled), the node finaliser
/// follows, and a shield neither lights nor has its meshes marked to cast
/// shadows.
pub fn shield_attach(class: u32) -> WeaponAttach {
    WeaponAttach {
        lit: false,
        casts_shadow: false,
        clears_part: class & 0x30 == 0,
        job: AttachJob::NodeFinal,
    }
}

/// Whether the take-in job can run this pass. The job reads the flags of
/// the scene node the attach made (not the item's record flags) and of
/// that node's part: until the node carries `0x2000000` and the part
/// `0x10000` the job is left queued and tried again on the next pass of
/// the job queue; only then does it take the item into the hand and leave
/// the queue. (That these two marks mean "loaded" is a guess from where
/// they are read; the bits themselves are the original's.)
pub fn take_in_ready(node_flags: u32, part_flags: u32) -> bool {
    node_flags & 0x0200_0000 != 0 && part_flags & 0x1_0000 != 0
}

/// The take-out a drag off the figure leaves behind: it exists only while
/// the hand's model slot is filled, and is armed so the off hand's weapon
/// or light is promoted into the main hand.
pub fn scheduled_take_out(hand_filled: bool, hand: usize) -> Option<TakeOut> {
    (hand_filled && hand < 2).then(|| TakeOut::scheduled(hand))
}

/// What attaching a node's item does when the job is over: the node leaves
/// the scene, the body is refreshed when neither out hand holds an item,
/// and a job whose mode has bit 1 restarts the stance and refreshes the
/// textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeFinal {
    pub detach_node: bool,
    pub refresh_body: bool,
    pub restart: bool,
}

/// The node finaliser's decisions.
pub fn node_final(mode: u32, both_hands_empty: bool) -> NodeFinal {
    NodeFinal {
        detach_node: true,
        refresh_body: both_hands_empty,
        restart: mode & 2 != 0,
    }
}

/// What a finished equipment job does. The three finalisers differ only in
/// which of these they run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FinishSteps {
    /// The body's effect attachments are cleared.
    pub detach_effects: bool,
    /// The body's apparel is rebuilt.
    pub rebuild_apparel: bool,
    /// The body's effect attachments are refreshed.
    pub refresh_effects: bool,
    /// The player's controls are refreshed.
    pub refresh_player: bool,
    /// The view's view-side callback runs.
    pub callback: bool,
    /// The view's job slot is cleared (the job is over).
    pub finish_job: bool,
}

/// Which finaliser runs after an equipment step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishKind {
    /// The doll's own finaliser: effects cleared and refreshed, the apparel
    /// rebuilt, the player's controls refreshed, the job finished.
    Doll,
    /// The other character's: the same on it, plus the view's callback and
    /// its job cleared, then the player's controls refreshed.
    Other,
    /// The light one: the apparel rebuilt and the job finished.
    Light,
}

/// The steps a finaliser runs, in the original's order.
pub fn finish(kind: FinishKind) -> FinishSteps {
    match kind {
        FinishKind::Doll => FinishSteps {
            detach_effects: true,
            rebuild_apparel: true,
            refresh_effects: true,
            refresh_player: true,
            callback: false,
            finish_job: true,
        },
        FinishKind::Other => FinishSteps {
            detach_effects: true,
            rebuild_apparel: true,
            refresh_effects: true,
            refresh_player: true,
            callback: true,
            finish_job: true,
        },
        FinishKind::Light => FinishSteps {
            detach_effects: false,
            rebuild_apparel: true,
            refresh_effects: false,
            refresh_player: false,
            callback: false,
            finish_job: true,
        },
    }
}

/// The item and flags a hand takes, as the hand's plan step sees it: a
/// weapon takes the lit and flipped bits of its flags, a light and a shield
/// take none.
pub fn hand_take(attach: &HandAttach) -> Attach {
    match attach.kind {
        AttachKind::Weapon => Attach::Weapon,
        AttachKind::Shield => Attach::Shield,
        AttachKind::Light => Attach::Light,
    }
}

/// The attach kind a modelled item takes in a hand, in the hand-scene
/// builder's order: a weapon section makes a weapon, except a weapon in the
/// off hand that also has a light section, which is that light; else a
/// shield section is a shield; else a light section is a light; else a
/// modelled item with none of those sections is attached as a weapon (the
/// job's mode word stays the weapon's `0`).
pub fn scene_attach(kind: ItemKind, hand: usize) -> AttachKind {
    if kind.weapon.is_some() {
        if hand == 1 && kind.light {
            AttachKind::Light
        } else {
            AttachKind::Weapon
        }
    } else if kind.shield {
        AttachKind::Shield
    } else if kind.light {
        AttachKind::Light
    } else {
        AttachKind::Weapon
    }
}

/// The plan that builds the doll's hand models, the scene job's own:
/// every hand whose target item differs from the item the body shows gets
/// a model when the item is modelled (`modelled`), attached by
/// [`scene_attach`]. A target of 0 changes the hand but builds no model, so
/// the hand's old model is left for the take-out that made room for it.
/// `swap` says the sets trade when the job is applied. `None` when neither
/// hand changes.
pub fn scene_plan<K, M>(
    current: &[Hand; 2],
    targets: &[Hand; 2],
    kind: K,
    modelled: M,
    swap: bool,
) -> Option<AttachPlan>
where
    K: Fn(u32) -> Option<ItemKind>,
    M: Fn(u32) -> bool,
{
    let mut plan = AttachPlan {
        changed: [false; 2],
        attach: [None; 2],
        targets: [targets[0].id, targets[1].id],
        swap,
    };
    for hand in 0..2 {
        plan.changed[hand] = targets[hand].id != current[hand].id;
        if !plan.changed[hand] || targets[hand].id == 0 || !modelled(targets[hand].id) {
            continue;
        }
        let Some(k) = kind(targets[hand].id) else {
            continue;
        };
        plan.attach[hand] = Some(HandAttach {
            id: targets[hand].id,
            flags: targets[hand].flags,
            kind: scene_attach(k, hand),
        });
    }
    plan.any().then_some(plan)
}

/// A hand model's scene node: the model is attached (the node's visible
/// mark), owned by the body, and its settle word says whether its item
/// counts as settled. The settle word is written by the part the item goes
/// into: -1 for an item with a weapon section (an off-hand weapon that also
/// lights included), the largest word for a shield; an item with only a
/// light section writes none (`None`: the node keeps its word).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandNode {
    pub attached: bool,
    pub settle: Option<i32>,
}

/// The scene node an attached hand model gets, by the item's sections in
/// the hand-scene builder's order (weapon, then shield).
pub fn hand_node(kind: ItemKind) -> HandNode {
    HandNode {
        attached: true,
        settle: if kind.weapon.is_some() {
            Some(WEAPON_SETTLE)
        } else if kind.shield {
            Some(SHIELD_SETTLE)
        } else {
            None
        },
    }
}

/// What releasing a hand's old model does: its scene node is detached, the
/// body is refreshed when neither of the record's two hands names an item
/// any more, and a take-out job whose mode carries [`RESTART_MODE`]
/// restarts the stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModelRelease {
    pub detach_node: bool,
    pub refresh_body: bool,
    pub restart: bool,
}

/// The release of a hand's old model, as the take-out's finaliser makes it.
/// `both_hands_empty` is the record's two out hand items both 0 after the
/// take-out, and `mode` the take-out job's mode word.
pub fn release_model(both_hands_empty: bool, mode: u32) -> ModelRelease {
    ModelRelease {
        detach_node: true,
        refresh_body: both_hands_empty,
        restart: mode & RESTART_MODE != 0,
    }
}

/// The two view counters a weapon-set swap writes (the swap control's
/// `0x60`-frame fades): the side the record's second-set flag leaves gets
/// [`SWAP_TIME`], the other 0.
pub fn swap_timings(second_set_out: bool) -> (i32, i32) {
    if second_set_out {
        (SWAP_TIME, 0)
    } else {
        (0, SWAP_TIME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::possession::{Carried, HELD, SECOND_SET};

    fn hand(id: u32, flags: u32) -> Hand {
        Hand { id, flags }
    }

    fn weapon(grip: u32) -> ItemKind {
        ItemKind {
            weapon: Some(grip),
            ..Default::default()
        }
    }

    fn torch() -> ItemKind {
        ItemKind {
            weapon: Some(0x10),
            light: true,
            ..Default::default()
        }
    }

    fn shield() -> ItemKind {
        ItemKind {
            shield: true,
            ..Default::default()
        }
    }

    fn light() -> ItemKind {
        ItemKind {
            light: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_plan_compares_each_hand_with_the_same_hand() {
        // A sword out, a shield stowed in the off hand; the main hand
        // differs.
        let out = [hand(5, 0), hand(0, 0)];
        let stowed = [hand(5, 0), hand(9, 0)];
        let plan = plan_attachments(&out, &stowed, |id| {
            Some(match id {
                5 => weapon(0x10),
                9 => shield(),
                _ => ItemKind::default(),
            })
        })
        .unwrap();
        assert_eq!(plan.changed, [false, true]);
        assert!(plan.any() && plan.swap);
        assert_eq!(plan.attach[0], None);
        assert_eq!(
            plan.attach[1],
            Some(HandAttach {
                id: 9,
                flags: 0,
                kind: AttachKind::Shield
            })
        );
        // Neither hand changes: no plan at all.
        assert_eq!(plan_attachments(&out, &out, |_| Some(weapon(0x10))), None);
    }

    #[test]
    fn the_attach_kind_reads_the_sections_in_order() {
        let out = [hand(0, 0), hand(0, 0)];
        let stowed = [hand(1, 0), hand(2, 0)];
        let plan = plan_attachments(&out, &stowed, |id| {
            Some(match id {
                // A weapon that also lights: a weapon in the main hand,
                // a light in the off hand.
                1 => torch(),
                2 => torch(),
                _ => ItemKind::default(),
            })
        })
        .unwrap();
        assert_eq!(plan.attach[0].unwrap().kind, AttachKind::Weapon);
        assert_eq!(plan.attach[1].unwrap().kind, AttachKind::Light);
        // An item with no attachable section changes the hand but attaches
        // nothing.
        let plan = plan_attachments(&out, &[hand(3, 0), hand(0, 0)], |_| {
            Some(ItemKind::default())
        })
        .unwrap();
        assert!(plan.changed[0]);
        assert_eq!(plan.attach[0], None);
        // A shield in the off hand is a shield even with a light section.
        let plan = plan_attachments(&out, &[hand(4, 0), hand(0, 0)], |_| {
            Some(ItemKind {
                shield: true,
                light: true,
                ..Default::default()
            })
        })
        .unwrap();
        assert_eq!(plan.attach[0].unwrap().kind, AttachKind::Shield);
    }

    #[test]
    fn applying_a_plan_detaches_then_attaches_and_trades_the_sets() {
        let mut p = Possessions {
            out: [hand(5, 0), hand(6, 0)],
            stowed: [hand(7, 0x0200_0000), hand(8, 0)],
            ..Default::default()
        };
        let plan = plan_from_record(&p.out, &[hand(7, 0), hand(8, 0)], |id| {
            Some(match id {
                7 => weapon(0x10),
                8 => shield(),
                _ => ItemKind::default(),
            })
        })
        .unwrap();
        assert_eq!(plan.changed, [true, true]);
        assert_eq!(plan.targets, [7, 8]);
        assert!(!plan.refresh_body());
        // A body with a weapon class bit keeps its weapon part. The body's
        // hand models hold 5 and 6.
        let steps = apply(&mut p, &plan, &[Some(5), Some(6)], 0, 0x10);
        assert_eq!(
            steps,
            vec![
                ApplyStep::Detach(0),
                ApplyStep::Detach(1),
                ApplyStep::Attach {
                    hand: 0,
                    kind: AttachKind::Weapon
                },
                ApplyStep::Attach {
                    hand: 1,
                    kind: AttachKind::Shield
                },
                ApplyStep::RefreshGrip,
                ApplyStep::RestartStance,
                ApplyStep::ClearAttachFlag,
            ]
        );
        // A redress plan does not trade the sets.
        assert_eq!(p.out, [hand(5, 0), hand(6, 0)]);
        // A swap plan does; the grip is always recomputed, the stance
        // restarted only outside the swap state. A body without weapon
        // class bits has its weapon parts cleared after the detaches, and
        // targets both 0 refresh the body after the attaches. The body
        // holds no models, so nothing detaches.
        let plan = AttachPlan {
            changed: [true, false],
            attach: [None, None],
            targets: [0, 0],
            swap: true,
        };
        assert!(plan.refresh_body());
        let steps = apply(&mut p, &plan, &[None, None], SWAP_STATE, 0x40);
        assert_eq!(
            steps,
            vec![
                ApplyStep::ClearWeaponPart,
                ApplyStep::RefreshBody,
                ApplyStep::RefreshGrip,
                ApplyStep::ClearAttachFlag,
                ApplyStep::ExchangeSets
            ]
        );
        assert_eq!(p.out, [hand(7, 0x0200_0000), hand(8, 0)]);
        assert_eq!(p.stowed, [hand(5, 0), hand(6, 0)]);
    }

    #[test]
    fn the_scene_builder_builds_a_model_per_changed_hand() {
        let current = [hand(5, 0), hand(0, 0)];
        let targets = [hand(5, 0), hand(9, 0)];
        let kinds = |id: u32| {
            Some(match id {
                9 => ItemKind {
                    shield: true,
                    ..Default::default()
                },
                _ => weapon(0x10),
            })
        };
        // The main hand keeps its item: only the off hand gets a model.
        let plan = scene_plan(&current, &targets, kinds, |_| true, true).unwrap();
        assert_eq!(plan.changed, [false, true]);
        assert_eq!(plan.attach[0], None);
        assert_eq!(plan.attach[1].unwrap().kind, AttachKind::Shield);
        assert!(!plan.refresh_body());
        // A model-less item changes the hand but builds nothing; the body
        // is not refreshed while a target names an item.
        let plan = scene_plan(&current, &targets, kinds, |id| id != 9, true).unwrap();
        assert!(plan.changed[1]);
        assert_eq!(plan.attach[1], None);
        assert!(!plan.refresh_body());
        // Both targets 0: the hands change to nothing and the body is
        // refreshed.
        let plan = scene_plan(&current, &[hand(0, 0), hand(0, 0)], kinds, |_| true, true).unwrap();
        assert_eq!(plan.changed, [true, false]);
        assert!(plan.refresh_body());
        // The off hand's weapon that also lights is built as that light.
        let plan = scene_plan(
            &current,
            &[hand(5, 0), hand(11, 0)],
            |_| Some(torch()),
            |_| true,
            true,
        )
        .unwrap();
        assert_eq!(plan.attach[1].unwrap().kind, AttachKind::Light);
        // Any modelled item with no section is a weapon.
        assert_eq!(scene_attach(ItemKind::default(), 0), AttachKind::Weapon);
        // Neither hand differs: no plan.
        assert_eq!(scene_plan(&current, &current, kinds, |_| true, true), None);
    }

    #[test]
    fn a_released_model_detaches_and_may_restart() {
        // Both hands empty and the job mode's bit 1: everything runs.
        assert_eq!(
            release_model(true, RESTART_MODE),
            ModelRelease {
                detach_node: true,
                refresh_body: true,
                restart: true
            }
        );
        assert!(!release_model(false, 0x800).refresh_body);
        assert!(!release_model(false, 0x800).restart);
        assert_eq!(hand_node(weapon(0x10)).settle, Some(-1));
        // An off-hand weapon that lights still settles as a weapon.
        assert_eq!(hand_node(torch()).settle, Some(-1));
        let shield_kind = ItemKind {
            shield: true,
            ..Default::default()
        };
        assert_eq!(hand_node(shield_kind).settle, Some(0x7FFF_FFFF));
        let lamp = ItemKind {
            light: true,
            ..Default::default()
        };
        assert_eq!(hand_node(lamp).settle, None);
        assert!(hand_node(lamp).attached);
        // The swap's counters sit on the side the second-set flag leaves.
        assert_eq!(swap_timings(true), (0x60, 0));
        assert_eq!(swap_timings(false), (0, 0x60));
    }

    #[test]
    fn a_swap_moves_the_marks_and_builds_the_plan() {
        let mut p = Possessions {
            out: [hand(5, 0), hand(0, 0)],
            stowed: [hand(9, 0), hand(9, 0)],
            carried: vec![
                Carried {
                    id: 5,
                    flags: HELD,
                    x: 0,
                    y: 0,
                },
                Carried {
                    id: 9,
                    flags: 0,
                    x: 0,
                    y: 0,
                },
                Carried {
                    id: 9,
                    flags: 0,
                    x: 0,
                    y: 0,
                },
            ],
            ..Default::default()
        };
        let plan =
            begin_swap(&mut p, |id| Some(weapon(if id == 9 { 0x20 } else { 0x10 }))).unwrap();
        // The flag toggles and the marks move at once.
        assert_eq!(p.flags & SECOND_SET, SECOND_SET);
        let marks: Vec<u32> = p.carried.iter().map(|c| c.flags).collect();
        assert_eq!(marks, vec![0, HELD, HELD]);
        // The sets have not traded yet.
        assert_eq!(p.out[0].id, 5);
        assert_eq!(plan.changed, [true, true]);
        assert_eq!(plan.attach[0].unwrap().id, 9);
        assert_eq!(plan.attach[1].unwrap().kind, AttachKind::Weapon);
        // A view with a job in flight refuses the request.
        assert!(!Busy(true).request());
        assert!(Busy(false).request());
    }

    const FIRST: HeldPlace = HeldPlace::Weapon { first: true };
    const LATER: HeldPlace = HeldPlace::Weapon { first: false };

    #[test]
    fn taking_out_clears_the_record_entry_that_names_the_item() {
        let mut p = Possessions {
            out: [hand(5, 0), hand(7, 0)],
            ..Default::default()
        };
        // The off hand's model holds the main hand's item: the other
        // hand's entry is cleared.
        let job = TakeOut {
            record: true,
            ..TakeOut::plain(1, 99)
        };
        let r = take_out(&mut p, &job, Some((5, FIRST)), None);
        assert!(r.done);
        assert_eq!(r.flags, 0);
        assert_eq!(p.out[0].id, 0);
        assert_eq!(p.out[1].id, 7);
        // An empty hand, a hand out of range, and a hand that already holds
        // the item the take-out makes room for do nothing.
        assert!(!take_out(&mut p, &TakeOut::plain(0, 5), None, None).done);
        assert!(!take_out(&mut p, &TakeOut::plain(2, 5), Some((5, FIRST)), None).done);
        assert!(!take_out(&mut p, &TakeOut::plain(1, 7), Some((7, LATER)), None).done);
        assert!(take_out(&mut p, &TakeOut::plain(1, 99), Some((7, LATER)), None).done);
        // A plain take-out keeps the record.
        let mut p = Possessions {
            out: [hand(5, 0x0200_0000), hand(0, 0)],
            ..Default::default()
        };
        let r = take_out(&mut p, &TakeOut::plain(0, 99), Some((5, FIRST)), None);
        assert!(r.done);
        assert_eq!(r.flags, 0x0200_0000);
        assert_eq!(p.out[0].id, 5);
    }

    #[test]
    fn an_off_hand_light_or_shield_clears_the_off_entry_unread() {
        // The record's off hand names another item: its flags are read and
        // it is cleared all the same.
        let mut p = Possessions {
            out: [hand(5, 0), hand(8, 0x40)],
            ..Default::default()
        };
        let job = TakeOut {
            mode: 4,
            record: true,
            ..TakeOut::plain(1, 99)
        };
        let r = take_out(&mut p, &job, Some((7, HeldPlace::Light)), None);
        assert!(r.done && r.light_finish);
        assert_eq!(r.flags, 0x40);
        assert_eq!(p.out, [hand(5, 0), hand(0, 0x40)]);
        // A shield never asks for the light finaliser.
        let mut p = Possessions {
            out: [hand(5, 0), hand(9, 0x10)],
            ..Default::default()
        };
        let r = take_out(&mut p, &job, Some((9, HeldPlace::Shield)), None);
        assert!(!r.light_finish);
        assert_eq!(r.flags, 0x10);
        assert_eq!(p.out[1].id, 0);
    }

    #[test]
    fn a_scheduled_take_out_promotes_a_weapon_or_a_light_but_not_a_shield() {
        let mut p = Possessions {
            out: [hand(5, 0x0100_0000), hand(7, 0x0200_0000)],
            ..Default::default()
        };
        let r = take_out(
            &mut p,
            &TakeOut::scheduled(0),
            Some((5, FIRST)),
            Some((7, torch())),
        );
        assert_eq!(r.promoted, Some(7));
        // The promoted item takes the off hand's flags.
        assert_eq!(p.out[0], hand(7, 0x0200_0000));
        assert_eq!(p.out[1].id, 0);
        assert!(!r.both_empty);
        // A shield is not promoted and stays in the off hand.
        let mut p = Possessions {
            out: [hand(5, 0), hand(9, 0)],
            ..Default::default()
        };
        let r = take_out(
            &mut p,
            &TakeOut::scheduled(0),
            Some((5, FIRST)),
            Some((9, shield())),
        );
        assert_eq!(r.promoted, None);
        assert_eq!(p.out, [hand(0, 0), hand(9, 0)]);
        // A main-hand weapon that is not the body's first weapon (the off
        // hand's was attached before it) is not followed by a promotion.
        let mut p = Possessions {
            out: [hand(5, 0), hand(7, 0)],
            ..Default::default()
        };
        let r = take_out(
            &mut p,
            &TakeOut::scheduled(0),
            Some((5, LATER)),
            Some((7, weapon(0x10))),
        );
        assert_eq!(r.promoted, None);
        assert_eq!(p.out, [hand(0, 0), hand(7, 0)]);
        // A take-out from the off hand never promotes, and both hands
        // empty is reported.
        let mut p = Possessions {
            out: [hand(5, 0), hand(7, 0)],
            ..Default::default()
        };
        let r = take_out(
            &mut p,
            &TakeOut::scheduled(1),
            Some((7, FIRST)),
            Some((5, torch())),
        );
        assert_eq!(r.promoted, None);
        assert_eq!(p.out[0].id, 5);
        let r = take_out(&mut p, &TakeOut::scheduled(0), Some((5, FIRST)), None);
        assert!(r.both_empty && r.done);
        // The job mode's bit 1 (the scheduled job's mode 7) asks for the
        // stance restart when the old model is released; a plain job's
        // mode 0 does not.
        assert!(!take_out(&mut p, &TakeOut::plain(0, 1), Some((5, FIRST)), None).restarts);
        assert!(take_out(&mut p, &TakeOut::scheduled(0), Some((5, FIRST)), None).restarts);
    }

    #[test]
    fn taking_an_item_into_the_hands_follows_the_branch_order() {
        let facts = |kind, main, off, class, off_wanted| IntoFacts {
            kind,
            main,
            off,
            class,
            off_wanted,
        };
        // Empty hands: the main hand, as a weapon.
        let Into { plan, kind, .. } =
            take_into_hands(facts(weapon(0x10), None, None, 0x80, false), 0);
        assert_eq!(plan.into, HoldInto::Hand(0));
        assert_eq!(kind, AttachKind::Weapon);
        assert!(plan.steps.is_empty());
        // A torch beside a sword: the off hand is freed and the torch is an
        // off-hand light.
        let Into { plan, kind, .. } =
            take_into_hands(facts(torch(), Some((5, false)), Some(9), 0x10, false), 0);
        assert_eq!(plan.steps, vec![HoldStep::Release(1)]);
        assert_eq!(plan.into, HoldInto::OffLight);
        assert_eq!(kind, AttachKind::Light);
        // A sword taken while a torch is the main hand: the torch moves,
        // and the record's off hand takes the job's flags — the attach job
        // keeps only the top bit of the dragged item's flags.
        let Into {
            plan,
            moved_light,
            flags,
            ..
        } = take_into_hands(
            facts(weapon(0x10), Some((9, true)), None, 0x80, false),
            0x8200_0000,
        );
        assert_eq!(plan.steps, vec![HoldStep::LightToOff]);
        assert_eq!(flags, 0x8000_0000);
        assert_eq!(
            moved_light,
            Some(Hand {
                id: 9,
                flags: 0x8000_0000
            })
        );
        // The mask keeps only the top bit, so a torch's lit bit is not
        // written into the record by the take-in.
        assert_eq!(
            take_into_hands(facts(torch(), None, None, 0x80, false), 0x0200_0000).flags,
            0
        );
    }

    #[test]
    fn a_take_in_writes_the_hand_it_reported() {
        let mut p = Possessions::default();
        let plan = hold_plan(
            0x10,
            false,
            Some(crate::possession::Holding {
                id: 1,
                light: false,
            }),
            None,
            0x10,
            true,
        );
        record_into(&mut p, &plan, 5, 0x0200_0000);
        assert_eq!(p.out[1], hand(5, 0x0200_0000));
        // The off-hand light writes the off hand even though the take-in
        // reported no hand.
        let plan = hold_plan(
            0x10,
            true,
            Some(crate::possession::Holding {
                id: 1,
                light: false,
            }),
            None,
            0x10,
            false,
        );
        record_into(&mut p, &plan, 6, 0);
        assert_eq!(p.out[1].id, 6);
        assert_eq!(p.out[0].id, 0);
    }

    #[test]
    fn a_shield_frees_the_off_hand_and_the_record_takes_it() {
        let mut p = Possessions {
            out: [hand(5, 0), hand(9, 0)],
            ..Default::default()
        };
        let taken = take_shield(Some((9, HeldPlace::Shield)), 11, 4, &mut p, 0x10);
        assert!(taken.frees_off);
        assert!(!taken.clears_part);
        assert_eq!(p.out[1], hand(11, 4));
        assert_eq!(p.out[0].id, 5);
        let mut p = Possessions::default();
        let taken = take_shield(None, 11, 0, &mut p, 0);
        assert!(!taken.frees_off && taken.clears_part);
        assert_eq!(p.out[1].id, 11);
        // A two-hander's model in the off hand: the take-out finds it in
        // the main hand's entry and clears it.
        let mut p = Possessions {
            out: [hand(9, 0), hand(0, 0)],
            ..Default::default()
        };
        take_shield(
            Some((9, HeldPlace::Weapon { first: true })),
            11,
            0,
            &mut p,
            0x20,
        );
        assert_eq!(p.out, [hand(0, 0), hand(11, 0)]);
    }

    #[test]
    fn attaches_by_kind_for_a_single_hand() {
        assert_eq!(attach_of(weapon(0x10), 0), Some(AttachKind::Weapon));
        assert_eq!(attach_of(torch(), 1), Some(AttachKind::Light));
        assert_eq!(attach_of(shield(), 1), Some(AttachKind::Shield));
        assert_eq!(attach_of(light(), 0), Some(AttachKind::Light));
        assert_eq!(attach_of(ItemKind::default(), 0), None);
        assert_eq!(
            hand_take(&HandAttach {
                id: 1,
                flags: 0,
                kind: AttachKind::Shield
            }),
            Attach::Shield
        );
    }

    #[test]
    fn redressing_attaches_the_record_over_what_the_hands_hold() {
        let p = Possessions {
            out: [hand(5, 0x0200_0000), hand(6, 0)],
            ..Default::default()
        };
        // The hands hold nothing: both hands are attached from the record.
        let plan = redress(&p, [None, None], |id| {
            Some(match id {
                5 => torch(),
                6 => shield(),
                _ => ItemKind::default(),
            })
        })
        .unwrap();
        assert_eq!(plan.changed, [true, true]);
        assert_eq!(plan.attach[0].unwrap().flags, 0x0200_0000);
        assert!(!plan.swap);
        // The main hand already holds its item: only the off hand changes.
        let plan = redress(&p, [Some(5), None], |_| Some(weapon(0x10))).unwrap();
        assert_eq!(plan.changed, [false, true]);
        // A two-hander held in both models is the main hand alone.
        let plan = redress(&p, [Some(6), Some(6)], |_| Some(weapon(0x20))).unwrap();
        assert_eq!(plan.changed, [true, false]);
        assert!(redress(&p, [Some(5), Some(6)], |_| Some(weapon(0x10))).is_none());
    }

    #[test]
    fn the_model_marks_and_the_finalisers() {
        assert_eq!(
            model_marks(0x0300_0000),
            ModelMarks {
                flipped: true,
                lit: true
            }
        );
        assert_eq!(
            model_marks(0x0100_0000),
            ModelMarks {
                flipped: true,
                lit: false
            }
        );
        let doll = finish(FinishKind::Doll);
        assert!(doll.detach_effects && doll.rebuild_apparel && doll.refresh_effects);
        assert!(doll.refresh_player && doll.finish_job && !doll.callback);
        let other = finish(FinishKind::Other);
        assert!(other.callback && other.finish_job && other.rebuild_apparel);
        let light = finish(FinishKind::Light);
        assert!(light.rebuild_apparel && light.finish_job);
        assert!(!light.detach_effects && !light.refresh_player && !light.callback);
    }

    #[test]
    fn the_model_attach_and_the_node_finaliser() {
        assert_eq!(
            weapon_attach(0x0200_0000, 0x10, false),
            WeaponAttach {
                lit: true,
                casts_shadow: false,
                clears_part: false,
                job: AttachJob::TakeIn
            }
        );
        assert_eq!(
            weapon_attach(0, 0x80, true),
            WeaponAttach {
                lit: false,
                casts_shadow: true,
                clears_part: true,
                job: AttachJob::TakeIn
            }
        );
        // A shield never lights or marks its meshes to cast shadows, and hands over to
        // the node finaliser whatever the class word says.
        assert_eq!(shield_attach(0x20).job, AttachJob::NodeFinal);
        assert!(!shield_attach(0x20).lit && !shield_attach(0x20).casts_shadow);
        assert!(!shield_attach(0x20).clears_part);
        assert!(shield_attach(0).clears_part);
        let finaliser = node_final(7, true);
        assert!(finaliser.detach_node && finaliser.refresh_body && finaliser.restart);
        assert!(!node_final(0, false).refresh_body);
        assert!(!node_final(0, true).restart);
    }

    #[test]
    fn a_take_in_waits_for_both_marks_and_a_scheduled_take_out_needs_a_model() {
        // The take-in job runs once the node and its part both carry their
        // marks; either missing keeps the job queued for the next pass.
        assert!(take_in_ready(0x0200_0000, 0x1_0000));
        assert!(!take_in_ready(0, 0x1_0000));
        assert!(!take_in_ready(0x0200_0000, 0));
        assert!(!take_in_ready(0x0200_0000, 0xFFFF));
        // A drag off the figure can only take out a hand that holds a
        // model, and the job is the armed one for that hand.
        assert_eq!(scheduled_take_out(false, 0), None);
        assert_eq!(scheduled_take_out(true, 0), Some(TakeOut::scheduled(0)));
        assert_eq!(scheduled_take_out(true, 1), Some(TakeOut::scheduled(1)));
        assert_eq!(scheduled_take_out(true, 2), None);
        let armed = scheduled_take_out(true, 1).unwrap();
        assert_eq!(armed.mode, 7);
        assert_eq!(armed.instead, None);
        assert!(armed.record && armed.scheduled);
    }
}
