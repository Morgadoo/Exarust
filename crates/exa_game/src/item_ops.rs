//! The controls' item operations outside the world drags: marking an item
//! used or destroying it, the item GUI's socket use and drag lifecycle,
//! the item factory's spawn helper, the deferred drop task, and dropping a
//! carried item into the world. Rules: `docs/PLAYER.md` (Using what the
//! cursor is over) and the item GUI notes.
//!
//! The original's node pointers become entity/node ids here, and the parts
//! that need the running scene take closures (`spawn`, `attach`, `drop`) so
//! the arithmetic, the branch order and the guards are pinned by tests.
//! Nothing here is on the running game's drop path yet: there is no
//! inventory window or item cursor to start it.

use bevy::math::Affine3A;

/// The item GUI's payload messages: an item dragged out of the inventory,
/// and a scene object dragged onto it.
pub const ITEM_PAYLOAD: u32 = 0xFB01;
pub const OBJECT_PAYLOAD: u32 = 0xFB02;
/// The usable section's kind of an AR crystal (the only item the socket use
/// takes), and the second kind byte that marks it.
pub const AR_KIND: u8 = 0x11;
/// The item GUI's drag-mode flags.
pub const GUI_DRAG_PLACE: u32 = 0x1;
pub const GUI_DRAG_CANCEL: u32 = 0x10;
pub const GUI_DRAG_END_HOLD: u32 = 0x30;
/// The player-controls drop mode word and its final drag mode.
pub const DROP_TO_SCENE: u32 = 0x1000;
pub const DRAG_MODE: u32 = 0x2000;
/// The item drop's drag y limit (cm): the original passes −100000.
pub const DROP_Y_LIMIT_CM: f32 = -100000.0;
/// "Game object has no physics: " (the error's message prefix).
pub const NO_PHYSICS: &str = "Game object has no physics: ";
/// "Content Error" (the error's title).
pub const CONTENT_ERROR: &str = "Content Error";
/// F97's usable kinds: 9 cleans, 10 repairs and 0x10 places the held item
/// in the picked slot.
pub const CLEAN_KIND: u8 = 9;
pub const REPAIR_KIND: u8 = 10;
pub const PLACE_KIND: u8 = 0x10;

/// The item GUI manager's entries as the link clear sees them: each slot's
/// item node, 0 when empty.
pub fn clear_node_links(links: &mut [u64], removed: u64) -> usize {
    let mut cleared = 0;
    for link in links {
        if *link == removed {
            *link = 0;
            cleared += 1;
        }
    }
    cleared
}

/// Marks an item used: the used byte in bits 16–23, ORed into the flags, as
/// the original ORs it (an old high byte is not cleared).
pub fn mark_used(flags: u32, used: u8) -> u32 {
    flags | (u32::from(used) << 16)
}

/// Destroys the used item after its use (F91): the current item is taken
/// out of the scene; else the container's last entry is removed. Returns
/// whether something was destroyed.
pub fn destroy_used(is_current: bool, entries: &mut Vec<u32>) -> bool {
    if is_current {
        return true;
    }
    entries.pop().is_some()
}

/// What using the held item on a GUI entry does (F97).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiUse {
    Nothing,
    Clean { item: u32 },
    Repair { item: u32 },
    Place { slot: usize, item: u32 },
}

/// The GUI use: the held item's kind against the picked entry. `slot` is
/// 1-based as the hit test returns it. A clean or repair needs the entry's
/// item to have a model; placing needs the entry to be the held item and
/// the panel to accept it.
pub fn gui_use(
    kind: u8,
    slot: usize,
    picked: u32,
    held: u32,
    has_model: bool,
    accepts: bool,
) -> GuiUse {
    match kind {
        CLEAN_KIND if has_model => GuiUse::Clean { item: picked },
        REPAIR_KIND if has_model => GuiUse::Repair { item: picked },
        PLACE_KIND if picked == held && accepts => GuiUse::Place {
            slot: slot.saturating_sub(1),
            item: picked,
        },
        _ => GuiUse::Nothing,
    }
}

/// Whether an AR crystal's drag starts the GUI use (F99): usable kind
/// `0x11`, byte 8 clear and byte 9 `0x11`; then the panel's selected entry
/// is taken out and the use begins. `selected` and `take` say whether the
/// panel had one and whether it could be taken.
pub fn start_gui_use(
    kind: u8,
    byte8: u8,
    byte9: u8,
    selected: bool,
    take: impl FnOnce() -> bool,
) -> bool {
    if kind != AR_KIND || byte8 != 0 || byte9 != AR_KIND || !selected {
        return false;
    }
    take()
}

/// What a GUI drag's end does (F74).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiDragEnd {
    /// The item was placed into the scene (flag 1, the placement succeeded).
    Placed,
    /// The drag was cancelled and the object's hold ends (flag 0x10 and
    /// mode 0x30).
    CancelledEndHold,
    Cancelled,
}

/// The GUI drag's end, from its mode flags and whether the object went into
/// the scene. A placement's success takes flag 1; without either the place
/// or the cancel flag, the drag is cancelled too.
pub fn gui_drag_end(mode: u32, placed: bool) -> GuiDragEnd {
    if placed && mode & GUI_DRAG_PLACE != 0 {
        return GuiDragEnd::Placed;
    }
    if mode & GUI_DRAG_CANCEL != 0 {
        if mode == GUI_DRAG_END_HOLD {
            return GuiDragEnd::CancelledEndHold;
        }
        return GuiDragEnd::Cancelled;
    }
    GuiDragEnd::Cancelled
}

/// What a GUI drag claim does (F71).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// No claim: the message is not a payload, the controls refuse drags or
    /// one is already registering.
    None,
    /// The claim starts; the mode becomes [`DROP_TO_SCENE`], and an item
    /// payload also asks for the drop.
    Started { drop: bool },
    /// Another drag is under way.
    Busy,
}

/// The GUI drag claim from the payload message, the control mode word and
/// whether a drag is already current. `refused` is the controls' `0x14d`
/// flag.
pub fn claim_gui_drag(message: u32, mode: u32, refused: bool, has_current: bool) -> Claim {
    if message != ITEM_PAYLOAD && message != OBJECT_PAYLOAD {
        return Claim::None;
    }
    if refused || mode & DROP_TO_SCENE != 0 {
        return Claim::None;
    }
    if has_current {
        return Claim::Busy;
    }
    Claim::Started {
        drop: message == ITEM_PAYLOAD,
    }
}

/// The item factory's spawn of one object (F101): create the model for
/// `(id, variant)`; when there is one, refresh it, add it to the scene and
/// attach it under the parent when that is not its own node. Returns the
/// new node.
pub fn spawn_object(
    id: u32,
    variant: u32,
    create: impl FnOnce(u32, u32) -> Option<u64>,
    attach: impl FnOnce(u64) -> bool,
) -> Option<u64> {
    let node = create(id, variant)?;
    attach(node);
    Some(node)
}

/// The item GUI's factory spawn (F102): an item/object payload spawns the
/// panel's selected item, mapped through the factory (the original's
/// selected entry at `0x90 + 0x10`, mapped by `0x100102700`, with the
/// factory's variant word). Returns the node.
pub fn spawn_from_gui(
    message: u32,
    selected: Option<u32>,
    variant: u32,
    map: impl FnOnce(u32) -> u32,
    create: impl FnOnce(u32, u32) -> Option<u64>,
    attach: impl FnOnce(u64) -> bool,
) -> Option<u64> {
    if message != ITEM_PAYLOAD && message != OBJECT_PAYLOAD {
        return None;
    }
    let selected = selected?;
    spawn_object(map(selected), variant, create, attach)
}

/// The deferred drop task's first word: the pointer's two coordinates and
/// the item node (F107 packs them zeroed, then the scene's 0x90 +0x10/+0x14
/// and the node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTask {
    pub cursor: (i32, i32),
    pub item: u64,
}

impl DropTask {
    /// A zeroed task with the cursor and the item, as the deferred call's
    /// argument list is filled.
    pub fn new(cursor: (i32, i32), item: u64) -> Self {
        Self { cursor, item }
    }
}

/// A queue of deferred drops, in place of the original's worker-thread
/// dispatch: tasks run in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropQueue {
    pub tasks: Vec<DropTask>,
}

impl DropQueue {
    /// Queues a drop (the original allocates and dispatches its task).
    pub fn push(&mut self, task: DropTask) {
        self.tasks.push(task);
    }

    /// Runs every queued drop, in order, and returns their outcomes.
    pub fn run(&mut self, mut drop: impl FnMut(DropTask) -> DropOutcome) -> Vec<DropOutcome> {
        self.tasks.drain(..).map(&mut drop).collect()
    }
}

/// The state of the item the drop task drops.
#[derive(Debug, Clone, PartialEq)]
pub struct DropItem {
    pub node: u64,
    pub name: String,
    /// The node is the scene node of a physical body.
    pub has_physics: bool,
    /// The placement flags (task+4, the item's `0x184` word).
    pub placement: u32,
    /// The node's local frame before the drop.
    pub local: Affine3A,
    /// The camera's frame the node is placed through.
    pub camera: Affine3A,
}

/// What a drop did.
#[derive(Debug, Clone, PartialEq)]
pub enum DropOutcome {
    /// The item went into the scene: its frame, its placement word and the
    /// drag mode; the drag's y limit is [`DROP_Y_LIMIT_CM`].
    Dropped {
        node: u64,
        world: Affine3A,
        placement: u32,
        y_limit_cm: f32,
    },
    /// The item had no physical body: the content error, and it was left
    /// out of the scene.
    ContentError { node: u64, message: String },
    /// The drop mode was not set: the item was taken out of the scene.
    Removed { node: u64 },
}

/// Drops the item (F106): the mode word must carry [`DROP_TO_SCENE`]. A
/// body-less item reports the content error ("Game object has no physics:
/// <name>", title [`CONTENT_ERROR`]); else the node's frame is taken
/// through the camera, its placement word stored, and it is dragged with
/// the y limit. The node's scene registration is the caller's.
pub fn drop_item(mode: u32, item: DropItem) -> DropOutcome {
    if mode & DROP_TO_SCENE == 0 {
        return DropOutcome::Removed { node: item.node };
    }
    if !item.has_physics {
        return DropOutcome::ContentError {
            node: item.node,
            message: format!("{NO_PHYSICS}{}", item.name),
        };
    }
    DropOutcome::Dropped {
        node: item.node,
        world: item.camera * item.local,
        placement: item.placement,
        y_limit_cm: DROP_Y_LIMIT_CM,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    #[test]
    fn a_used_mark_ors_the_high_byte() {
        assert_eq!(mark_used(0x0000_0000, 0xFF), 0x00FF_0000);
        assert_eq!(mark_used(0x00AB_0001, 0x02), 0x00AB_0001 | 0x0002_0000);
        assert_eq!(mark_used(0x1234_5678, 0), 0x1234_5678);
    }

    #[test]
    fn cleared_node_links_count_their_slots() {
        let mut links = [1, 7, 7, 0, 7];
        assert_eq!(clear_node_links(&mut links, 7), 3);
        assert_eq!(links, [1, 0, 0, 0, 0]);
        assert_eq!(clear_node_links(&mut links, 9), 0);
    }

    #[test]
    fn destroying_used_items() {
        let mut entries = vec![10, 20];
        assert!(destroy_used(false, &mut entries));
        assert_eq!(entries, vec![10]);
        assert!(destroy_used(true, &mut entries));
        assert_eq!(entries, vec![10]);
        assert!(destroy_used(false, &mut entries));
        assert!(!destroy_used(false, &mut entries));
    }

    /// Clean and repair need a model; a place needs the held item and the
    /// panel's acceptance; the slot is made 0-based.
    #[test]
    fn the_gui_use_of_a_held_item() {
        assert_eq!(gui_use(9, 1, 44, 7, true, true), GuiUse::Clean { item: 44 });
        assert_eq!(gui_use(9, 1, 44, 7, false, true), GuiUse::Nothing);
        assert_eq!(
            gui_use(10, 2, 44, 7, true, true),
            GuiUse::Repair { item: 44 }
        );
        assert_eq!(
            gui_use(0x10, 3, 7, 7, true, true),
            GuiUse::Place { slot: 2, item: 7 }
        );
        assert_eq!(gui_use(0x10, 3, 7, 8, true, true), GuiUse::Nothing);
        assert_eq!(gui_use(0x10, 3, 7, 7, true, false), GuiUse::Nothing);
        assert_eq!(gui_use(8, 1, 44, 7, true, true), GuiUse::Nothing);
    }

    /// The AR crystal's use starts only with kind 0x11, byte 8 clear, byte 9
    /// 0x11 and a selected panel entry.
    #[test]
    fn the_ar_crystal_starts_the_gui_use() {
        let mut taken = false;
        assert!(start_gui_use(0x11, 0, 0x11, true, || {
            taken = true;
            true
        }));
        assert!(taken);
        assert!(!start_gui_use(0x11, 0, 0x11, false, || true));
        assert!(!start_gui_use(0x11, 1, 0x11, true, || true));
        assert!(!start_gui_use(0x11, 0, 0x10, true, || true));
        assert!(!start_gui_use(0x10, 0, 0x11, true, || true));
        assert!(!start_gui_use(0x11, 0, 0x11, true, || false));
    }

    /// A placed drag with flag 1 succeeds; the cancel flag ends it, ending
    /// the object's hold only at mode 0x30; anything else cancels.
    #[test]
    fn the_gui_drag_end() {
        assert_eq!(gui_drag_end(0x1, true), GuiDragEnd::Placed);
        assert_eq!(gui_drag_end(0x1, false), GuiDragEnd::Cancelled);
        assert_eq!(gui_drag_end(0x10, false), GuiDragEnd::Cancelled);
        assert_eq!(gui_drag_end(0x30, false), GuiDragEnd::CancelledEndHold);
        assert_eq!(gui_drag_end(0x30, true), GuiDragEnd::CancelledEndHold);
        assert_eq!(gui_drag_end(0x0, false), GuiDragEnd::Cancelled);
    }

    #[test]
    fn the_gui_drag_claim() {
        assert_eq!(claim_gui_drag(0x1234, 0, false, false), Claim::None);
        assert_eq!(
            claim_gui_drag(ITEM_PAYLOAD, 0, false, false),
            Claim::Started { drop: true }
        );
        assert_eq!(
            claim_gui_drag(OBJECT_PAYLOAD, 0, false, false),
            Claim::Started { drop: false }
        );
        assert_eq!(claim_gui_drag(ITEM_PAYLOAD, 0, true, false), Claim::None);
        assert_eq!(
            claim_gui_drag(ITEM_PAYLOAD, DROP_TO_SCENE, false, false),
            Claim::None
        );
        assert_eq!(claim_gui_drag(ITEM_PAYLOAD, 0, false, true), Claim::Busy);
    }

    /// The factory spawns only a created model and attaches it; the GUI path
    /// requires a payload and a selected entry.
    #[test]
    fn spawning_items_from_the_factory_and_the_gui() {
        let mut attached = false;
        assert_eq!(
            spawn_object(
                5,
                1,
                |id, variant| {
                    assert_eq!((id, variant), (5, 1));
                    Some(77)
                },
                |node| {
                    assert_eq!(node, 77);
                    attached = true;
                    true
                }
            ),
            Some(77)
        );
        assert!(attached);
        assert_eq!(spawn_object(5, 1, |_, _| None, |_| true), None);
        let node = spawn_from_gui(
            ITEM_PAYLOAD,
            Some(12),
            4,
            |id| id + 100,
            |id, v| {
                assert_eq!((id, v), (112, 4));
                Some(3)
            },
            |_| true,
        );
        assert_eq!(node, Some(3));
        assert_eq!(
            spawn_from_gui(0x1234, Some(12), 4, |id| id, |_, _| Some(3), |_| true),
            None
        );
        assert_eq!(
            spawn_from_gui(OBJECT_PAYLOAD, None, 4, |id| id, |_, _| Some(3), |_| true),
            None
        );
    }

    /// The deferred queue runs its drops in the order they were queued.
    #[test]
    fn the_drop_queue_runs_in_order() {
        let mut queue = DropQueue::default();
        queue.push(DropTask::new((10, 20), 1));
        queue.push(DropTask::new((30, 40), 2));
        let seen = queue.run(|task| DropOutcome::Removed { node: task.item });
        assert_eq!(
            seen,
            vec![
                DropOutcome::Removed { node: 1 },
                DropOutcome::Removed { node: 2 }
            ]
        );
        assert!(queue.tasks.is_empty());
    }

    /// A drop without the mode takes the item out; a body-less item reports
    /// "Game object has no physics: <name>"; a physical one goes through the
    /// camera transform with its placement and the −100000 cm drag limit.
    #[test]
    fn dropping_an_item_into_the_world() {
        let item = DropItem {
            node: 5,
            name: "Sword A01".into(),
            has_physics: true,
            placement: 0x0012_0000,
            local: Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0)),
            camera: Affine3A::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        };
        assert_eq!(drop_item(0, item.clone()), DropOutcome::Removed { node: 5 });
        let error = drop_item(
            DROP_TO_SCENE,
            DropItem {
                has_physics: false,
                ..item.clone()
            },
        );
        assert_eq!(
            error,
            DropOutcome::ContentError {
                node: 5,
                message: "Game object has no physics: Sword A01".into()
            }
        );
        let DropOutcome::Dropped {
            world,
            placement,
            y_limit_cm,
            ..
        } = drop_item(DROP_TO_SCENE, item)
        else {
            panic!("not dropped");
        };
        assert_eq!(world.translation, Vec3::new(11.0, 2.0, 3.0).into());
        assert_eq!(placement, 0x0012_0000);
        assert_eq!(y_limit_cm, -100000.0);
    }
}
