//! The save's level section: the current level written into the save under
//! its locale name, with the live local tier and the live placed objects,
//! so a load reads the level from the save's own section and finds the
//! objects where the player left them.
//!
//! **What the original writes.** After the world databases, the save
//! exports the game's whole scene with the save switches (the story's
//! exporter adds the settings chunk) and stores it as the package section
//! named by the scene's locale. The exporter's local-database chunk
//! (`0xDBC0`) holds the **live local tier**: its `0xDBDB` records (the
//! tier's writer, the slot after the last entry included) and its `0xDBCD`
//! string library, written when the walk found a placed object and the
//! tier holds a record, then the placed objects (`0xDBCE`) as the scene
//! holds them now.
//!
//! **What Exarust writes.** Its scene model has no converter back into the
//! exporter's scene, so the export starts from the section the level was
//! read from ([`exa_world::LevelSource`]) and writes the live state over
//! it: the local tier and its library from the live database, and the
//! placed objects from the live entities: an object the load spawned keeps
//! its place in the list with its live id, condition and placement (a body
//! that moved takes the body's motion since it was registered); one the
//! load spawned that is gone (picked up, broken) leaves the list with the
//! objects inside it, and the later objects' container numbers close up; one
//! the load never spawned (a record without a model, another slot's) stays
//! as the section had it; an object dropped into the world since is added
//! at the end. A story save also writes the scene's characters: the save
//! first flushes every scene character's live possessions and health into
//! its record in its store ([`LevelSaveScene::flush_characters`]), then the
//! section's placements follow the live scene ([`characters_after`]) and its
//! character database is the live local store
//! ([`crate::world_characters::LocalCharacters`]); each placement carries
//! its live body ([`live_state`]: a new corpse's pose tree, a driven
//! character's pose tree and motion state, an awake guardian's AI home).
//! The player's body is left out (its spawn sets the exporter's owned mark),
//! except the dead player's body once the story's respawn or exit cleared
//! that mark ([`crate::left_behind`]): it is written as a dead character
//! under its left-behind record.
//! The props (objects from the object libraries the level places) are
//! written where they stand ([`props_after`]): each prop root's live
//! placement in its parent's frame, a prop over a moved free body taking
//! the body's motion since it was registered. A body hung from the world by
//! its links (a door's leaf, a lever's arm, a chain) moves neither the
//! placed object nor the prop it hangs in.
//! The part records of an object's or prop's owned bodies and cloth are the
//! live ones ([`part_records`]): a hung leaf's, an arm's or a chain link's
//! live world frame, a moved cloth's live particles, so a door left open
//! loads open. A mechanism's placement word is its live object word
//! ([`LiveWord`]: a door's stored word with its latch, mode and countdown,
//! a lever's word with an armed return's time, a push surface's state).
//! [Exarust's: the settings chunk's live words are the loaded section's;
//! the living spawn does not restore a saved pose tree.]
//!
//! The original's character section (`0xCAC00001`): each character's health
//! is stored into its record, the local character database is flushed and
//! written (`0xCECDDB00`) when it holds a record, then each character's
//! placement (`0xCAC0CA00`: the record's id, the body's x, floor and z, the
//! heading and the condition word, then the pose tree for a dead body, and
//! in save mode the AI home of an awake guardian and, for an AI in its
//! attached mode, the pose tree and the motion state).
//!
//! Evidence: the save unit's save routine (the section order, the
//! exporter's call and the locale name) and the exporter's scene walk and
//! local-database chunk (`docs/formats/SAVES.md`, Saving; `RFC.md`).

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_formats::characters::{
    AiHome, ID_MASK, MotionState, Placement, PoseRecord, SavedPose, WORLD_DB,
};
use exa_formats::rfc::{PlacedObject, Scene};
use exa_formats::rfc_level::LevelRaw;

use crate::level_transition::{SettingsState, export_level_scene};

/// A placed object as it stands in the running scene.
#[derive(Debug, Clone, PartialEq)]
pub struct LivePlacement {
    /// Its place in the loaded section's list; `None` for an object put
    /// into the world since (a drop).
    pub index: Option<u32>,
    /// Its database id, tier tag included.
    pub id: u32,
    /// The condition byte of its placement word.
    pub condition: u8,
    /// Its placement: three rotation rows and the translation (metres).
    pub rows: [[f32; 3]; 4],
    /// A mechanism's live object word (a door's or sliding leaf's stored
    /// word, a lever's word with an armed return's time, a push surface's
    /// state byte), which the placement record stores as the original's
    /// object word; `None` keeps the stored word with the live condition
    /// byte.
    pub word: Option<LiveWord>,
    /// Its model's nodes as they stand, which its part records are written
    /// from ([`part_records`]); `None` keeps the section's records.
    pub parts: Option<Vec<LivePart>>,
}

/// A mechanism's live object word: the stored word's bits under `keep`
/// stay, `value` fills the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveWord {
    pub keep: u32,
    pub value: u32,
}

impl LiveWord {
    /// The placement word `stored` becomes.
    pub fn over(self, stored: u32) -> u32 {
        (stored & self.keep) | (self.value & !self.keep)
    }
}

/// What the placed-object pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlacementCounts {
    /// Objects the load spawned that are still in the scene.
    pub kept: usize,
    /// Of those, the ones whose placement changed.
    pub moved: usize,
    /// Of those, the ones that moved more than a centimetre.
    pub moved_far: usize,
    /// Objects the load spawned that are gone, with the objects inside them.
    pub removed: usize,
    /// Objects the load never spawned, kept as the section had them.
    pub unspawned: usize,
    /// Objects added since the load.
    pub added: usize,
    /// Kept objects whose part records the live scene rewrote.
    pub parts: usize,
}

/// A placement's rows from a world transform.
pub fn rows_of(affine: Affine3A) -> [[f32; 3]; 4] {
    [
        affine.matrix3.x_axis.to_array(),
        affine.matrix3.y_axis.to_array(),
        affine.matrix3.z_axis.to_array(),
        affine.translation.to_array(),
    ]
}

/// How far two placements are apart: the largest component difference.
fn rows_apart(a: &[[f32; 3]; 4], b: &[[f32; 3]; 4]) -> f32 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

/// A live placement closer than this to the stored one keeps the stored
/// rows, so an object nothing touched is written back unchanged: the live
/// transform is the stored one taken through single-precision matrices
/// and a sleeping body's reconstructed frame, which leaves noise of a few
/// tenths of a millimetre on most loose objects [Exarust's: the original
/// writes every node's live matrix].
const UNMOVED: f32 = 1e-3;

/// The section's placed objects after the live scene: see the module notes.
/// `nodes` and `props` are the section's scene node and prop counts, which
/// number the containers an object's slot word names (an object's number is
/// `nodes + props + index`).
pub fn placed_after(
    placed: &[PlacedObject],
    nodes: usize,
    props: usize,
    spawned: &[u32],
    live: &[LivePlacement],
) -> (Vec<PlacedObject>, PlacementCounts) {
    let mut counts = PlacementCounts::default();
    let first_object = nodes + props;
    let by_index: std::collections::HashMap<u32, &LivePlacement> = live
        .iter()
        .filter_map(|p| p.index.map(|index| (index, p)))
        .collect();
    let spawned: std::collections::HashSet<u32> = spawned.iter().copied().collect();
    // Gone: spawned and no longer standing; then whatever was inside them.
    let mut gone: Vec<bool> = (0..placed.len())
        .map(|i| {
            let i = i as u32;
            spawned.contains(&i) && !by_index.contains_key(&i)
        })
        .collect();
    let container = |object: &PlacedObject| -> Option<usize> {
        let number = usize::try_from(object.slot).ok()?;
        number.checked_sub(first_object)
    };
    loop {
        let mut changed = false;
        for (i, object) in placed.iter().enumerate() {
            if !gone[i] && container(object).is_some_and(|c| gone.get(c) == Some(&true)) {
                gone[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Each kept object's new index, to renumber the slot words.
    let mut new_index = Vec::with_capacity(placed.len());
    let mut next = 0usize;
    for &g in &gone {
        new_index.push((!g).then_some(next));
        if !g {
            next += 1;
        }
    }
    let mut out = Vec::with_capacity(placed.len());
    for (i, object) in placed.iter().enumerate() {
        if gone[i] {
            counts.removed += 1;
            continue;
        }
        let mut object = object.clone();
        if let Some(c) = container(&object)
            && let Some(Some(at)) = new_index.get(c)
        {
            object.slot = (first_object + at) as i32;
        }
        match by_index.get(&(i as u32)) {
            Some(live) => {
                counts.kept += 1;
                object.id = live.id;
                object.flags = match live.word {
                    Some(word) => word.over(object.flags),
                    None => (object.flags & !0x00FF_0000) | (u32::from(live.condition) << 16),
                };
                if let Some(records) = live
                    .parts
                    .as_deref()
                    .and_then(|parts| part_records(&object.attachments, parts))
                {
                    counts.parts += 1;
                    object.attachments = records;
                }
                if rows_apart(&live.rows, &object.transform) > UNMOVED {
                    let travel = Vec3::from(live.rows[3]).distance(Vec3::from(object.transform[3]));
                    if travel > 0.01 {
                        counts.moved_far += 1;
                    }
                    object.transform = live.rows;
                    counts.moved += 1;
                }
            }
            None => counts.unspawned += 1,
        }
        out.push(object);
    }
    for live in live.iter().filter(|p| p.index.is_none()) {
        counts.added += 1;
        let mut object = exa_world::dropped_object(live.id, u32::from(live.condition) << 16);
        object.transform = live.rows;
        out.push(object);
    }
    (out, counts)
}

/// A prop (an object from the object libraries the level places) as it
/// stands in the running scene: its place in the section's prop list and
/// its placement in its parent's frame (the parent prop's, or the level's
/// for a prop at the root), three rotation rows and the translation
/// (metres), as the section's prop record stores it.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveProp {
    pub index: u32,
    pub rows: [[f32; 3]; 4],
    /// Its model's nodes as they stand, which its part records are written
    /// from ([`part_records`]); `None` keeps the section's records.
    pub parts: Option<Vec<LivePart>>,
}

/// What the prop pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PropCounts {
    /// Props the scene still holds.
    pub kept: usize,
    /// Of those, the ones whose placement changed.
    pub moved: usize,
    /// Of those, the ones that moved more than a centimetre.
    pub moved_far: usize,
    /// Props whose part records the live scene rewrote.
    pub parts: usize,
}

/// The section's props after the live scene: each prop the scene holds
/// takes its live placement (the original's prop record writes the prop
/// node's live matrix), unless it stands within [`UNMOVED`] of the stored
/// one, which is kept. A prop the scene does not hold keeps its record.
pub fn props_after(props: &mut [exa_formats::level::Prop], live: &[LiveProp]) -> PropCounts {
    let mut counts = PropCounts::default();
    for prop in live {
        let Some(stored) = props.get_mut(prop.index as usize) else {
            continue;
        };
        counts.kept += 1;
        if let Some(records) = prop
            .parts
            .as_deref()
            .and_then(|parts| part_records(&stored.attachments, parts))
        {
            counts.parts += 1;
            stored.attachments = records;
        }
        if rows_apart(&prop.rows, &stored.transform) > UNMOVED {
            let travel = Vec3::from(prop.rows[3]).distance(Vec3::from(stored.transform[3]));
            counts.moved += 1;
            counts.moved_far += usize::from(travel > 0.01);
            stored.transform = prop.rows;
        }
    }
    counts
}

/// One node of a placed object's or prop's model as it stands now, for
/// [`part_records`]: its tree place and name, the attachment its model node
/// carries, its live world frame (metres) and what moved since the load.
#[derive(Debug, Clone, PartialEq)]
pub struct LivePart {
    pub name: String,
    /// Its parent's index in the list; `None` for a root (the first root
    /// stands for the object's node).
    pub parent: Option<usize>,
    pub kind: exa_world::saved_parts::PartKind,
    /// Its live world frame (metres).
    pub world: Affine3A,
    /// It moved against its parent since the load (by more than
    /// [`UNMOVED`]).
    pub moved: bool,
    /// For a cloth that moved off its start: every position element of its
    /// mesh in the level's frame (metres), the live particles.
    pub cloth: Option<Vec<[f32; 3]>>,
}

/// The part records the original's exporter writes for a placed object or a
/// prop, over the live nodes: walking the object node's children in order,
/// a body's node writes a transform record (its name, its word, its live
/// world frame) with its own children's records nested in it, a cloth's
/// node writes its live particles, a node with no attachment is passed over
/// with its subtree, and an emitter ends its siblings' records.
///
/// [Exarust's: a record the section stored is written back unchanged while
/// its node stands within [`UNMOVED`] of it (the live frame is the stored
/// one through single-precision matrices), and an owned node with no stored
/// record that has not moved against its parent writes none, since the
/// load puts it back where it is from its parent alone (the original writes
/// a record for every owned body and cloth; the load would then copy every
/// model that holds one). A record's word is the stored one (0 in every
/// shipped record; 0 for a new one).]
///
/// Returns `None` when nothing moved (the stored records are written as they
/// are), or when they hold a placement tree (`0x3DE0EC00`, none shipped),
/// which the exporter never writes back.
pub fn part_records(
    stored: &[exa_formats::level::PropAttachment],
    parts: &[LivePart],
) -> Option<Vec<exa_formats::level::PropAttachment>> {
    use exa_formats::level::PropAttachment;
    use exa_world::saved_parts::PartKind;
    fn has_tree(list: &[PropAttachment]) -> bool {
        list.iter().any(|r| match r {
            PropAttachment::Tree { .. } => true,
            PropAttachment::Transform { children, .. } => has_tree(children),
            PropAttachment::MeshPositions { .. } => false,
        })
    }
    fn walk(
        at: usize,
        stored: &[PropAttachment],
        parts: &[LivePart],
        depth: usize,
    ) -> (Vec<PropAttachment>, bool) {
        let mut out = Vec::new();
        let mut changed = false;
        if depth > parts.len() {
            return (out, false);
        }
        for (c, part) in parts.iter().enumerate() {
            if c == at || part.parent != Some(at) {
                continue;
            }
            let record = stored.iter().find(|r| {
                r.name() == part.name
                    && matches!(
                        (r, part.kind),
                        (PropAttachment::Transform { .. }, PartKind::Body)
                            | (PropAttachment::MeshPositions { .. }, PartKind::Cloth)
                    )
            });
            match part.kind {
                PartKind::None => continue,
                PartKind::Emitter => break,
                PartKind::Body => {
                    let (word, kept, inner) = match record {
                        Some(PropAttachment::Transform {
                            word,
                            transform,
                            children,
                            ..
                        }) => (*word, Some(*transform), children.as_slice()),
                        _ => (0, None, &[][..]),
                    };
                    let (children, inner_changed) = walk(c, inner, parts, depth + 1);
                    let live = rows_of(part.world);
                    let transform = match kept {
                        Some(rows) if rows_apart(&rows, &live) <= UNMOVED => rows,
                        Some(_) => {
                            changed = true;
                            live
                        }
                        None if part.moved || !children.is_empty() => {
                            changed = true;
                            live
                        }
                        None => continue,
                    };
                    changed |= inner_changed;
                    out.push(PropAttachment::Transform {
                        name: part.name.clone(),
                        word,
                        transform,
                        children,
                    });
                }
                PartKind::Cloth => {
                    let word = match record {
                        Some(PropAttachment::MeshPositions { word, .. }) => *word,
                        _ => 0,
                    };
                    match (&part.cloth, record) {
                        (Some(positions), _) => {
                            changed = true;
                            out.push(PropAttachment::MeshPositions {
                                name: part.name.clone(),
                                word,
                                positions: positions.clone(),
                            });
                        }
                        (None, Some(record)) => out.push(record.clone()),
                        (None, None) => {}
                    }
                }
            }
        }
        (out, changed)
    }
    if has_tree(stored) {
        return None;
    }
    let root = parts.iter().position(|p| p.parent.is_none())?;
    let (records, changed) = walk(root, stored, parts, 0);
    // Once anything changed the list is the walk's: a stored record it did
    // not reach (no such node, or past an emitter) is left out, as the
    // original's exporter writes only what the live nodes hold.
    changed.then_some(records)
}

/// A debug helper for the saved props' evidence (`EXA_PROP_NUDGE=dx,dy,dz`,
/// metres): once the player stands in a level, the prop root nearest to the
/// player at the level's root is logged with its section index and place,
/// then moved by the offset (an offset of zero only logs it, which a load
/// of the save uses to show where the prop came back).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PropNudge {
    pub offset: Vec3,
    pub done: bool,
}

impl PropNudge {
    /// `dx,dy,dz` in metres.
    pub fn parse(text: &str) -> Option<Self> {
        let v: Vec<f32> = text
            .split(',')
            .map(|w| w.trim().parse().ok())
            .collect::<Option<_>>()?;
        let [x, y, z] = v[..] else {
            return None;
        };
        Some(Self {
            offset: Vec3::new(x, y, z),
            done: false,
        })
    }
}

/// Runs the [`PropNudge`] once.
pub fn nudge_prop(
    mut nudge: ResMut<PropNudge>,
    players: Query<&GlobalTransform, With<crate::player::PlayerBody>>,
    mut props: Query<(
        &exa_world::PropIndex,
        &Name,
        &GlobalTransform,
        &mut Transform,
        &ChildOf,
    )>,
    roots: Query<(), With<exa_world::LevelRoot>>,
) {
    if nudge.done {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let at = player.translation();
    let Some((index, name, global, mut transform, _)) = props
        .iter_mut()
        .filter(|(.., parent)| roots.contains(parent.parent()))
        .min_by(|a, b| {
            let da = a.2.translation().distance_squared(at);
            let db = b.2.translation().distance_squared(at);
            da.total_cmp(&db)
        })
    else {
        return;
    };
    nudge.done = true;
    info!(
        "prop nudge: prop {} ({}) at {:.3} m, {:.2} m from the player, moved by {:.3}",
        index.0,
        name.as_str(),
        global.translation(),
        global.translation().distance(at),
        nudge.offset
    );
    transform.translation += nudge.offset;
}

/// A character as it stands in the running scene, for the save's
/// character section.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveCharacter {
    /// Its record's id (the world tag for a world record).
    pub id: u32,
    /// Its feet, metres.
    pub position: [f32; 3],
    /// Its heading (Exarust's yaw).
    pub yaw: f32,
    /// Its body is dead.
    pub dead: bool,
    /// Its body's live pose tree ([`live_pose_tree`]), when it has a
    /// physical body: written for a dead body and with the motion state.
    pub pose: Option<Vec<PoseRecord>>,
    /// Its current motion and that motion's progress, for a character an
    /// AI drives (the original's AI in its attached mode 1).
    pub motion: Option<MotionState>,
    /// Its AI home, for an awake guardian (a race word whose `0xff00` pair
    /// is `0x1000`, statue byte clear) that holds one.
    pub ai_home: Option<AiHome>,
}

/// The pose tree a save writes for a body: its root part's world frame,
/// then each child part's, depth first, each record with its bone number
/// and its child count, as the exporter's pose walk writes the part nodes
/// (`0xCAC0EB00`). `parts` are the body's parts as (bone, parent part,
/// world transform); the root is bone 1's part, else the first part
/// without a parent. Empty for a body without parts.
pub fn live_pose_tree(parts: &[(usize, Option<usize>, Affine3A)]) -> Vec<PoseRecord> {
    fn visit(
        at: usize,
        parts: &[(usize, Option<usize>, Affine3A)],
        out: &mut Vec<PoseRecord>,
        depth: usize,
    ) {
        let children: Vec<usize> = (0..parts.len())
            .filter(|&k| k != at && parts[k].1 == Some(at))
            .collect();
        out.push(PoseRecord {
            frame: exa_character::pose::frame_of_affine(&parts[at].2),
            word: parts[at].0 as u32,
            children: children.len() as u32,
        });
        // A malformed parent chain cannot loop the walk.
        if depth > parts.len() {
            return;
        }
        for child in children {
            visit(child, parts, out, depth + 1);
        }
    }
    let root = parts
        .iter()
        .position(|p| p.0 == 1 && p.1.is_none())
        .or_else(|| parts.iter().position(|p| p.1.is_none()));
    let mut out = Vec::new();
    if let Some(root) = root {
        visit(root, parts, &mut out, 0);
    }
    out
}

/// The motion state a save stores: the current motion and its progress.
/// A load starts the motion with no blend and puts its progress back;
/// an attack motion (5 to 12) is replaced by the defence stance (3) with
/// no progress, as the original's character builder does.
pub fn loaded_motion(saved: MotionState) -> MotionState {
    if (5..=12).contains(&saved.state) {
        MotionState {
            state: DEFENCE_MOTION,
            value: 0.0,
        }
    } else {
        saved
    }
}

/// The defence stance's motion number.
pub const DEFENCE_MOTION: u32 = 3;

/// The race word pair (`0xff00`) whose character saves its AI home.
pub const HOME_RACE_PAIR: u32 = 0x1000;

/// What the character pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharacterCounts {
    /// Placements whose character still stands in the scene.
    pub kept: usize,
    /// Of those, the ones written with the live place (or death).
    pub moved: usize,
    /// Placements whose record left the level (a follower that took a
    /// door), left out.
    pub removed: usize,
    /// Placements with no live character, kept as the section had them.
    pub unspawned: usize,
    /// Characters with no placement in the section (an arrived follower),
    /// added at the end.
    pub added: usize,
}

/// A live character closer than this (metres, any axis) to its stored
/// placement keeps the stored one, so a character that only idled is
/// written back unchanged [Exarust's: the original writes every
/// character's live body position and heading; Exarust's standing bodies
/// sway by a few centimetres].
const CHARACTER_UNMOVED: f32 = 0.1;

/// Whether two ids name the same record: the same store (the world bit)
/// and the same low 28 bits.
fn same_record(a: u32, b: u32) -> bool {
    a & WORLD_DB == b & WORLD_DB && a & ID_MASK == b & ID_MASK
}

/// The section's character placements after the live scene, as the
/// exporter's walk writes the scene's characters: each stored placement
/// whose record's character stands in the scene (the nearest of that
/// record's) takes its live place, heading and death when it moved; one
/// with no live character stays as stored unless `gone` says its record
/// left the level's store (a follower flushed at a door: its node left the
/// scene); a live character with no placement is added at the end with its
/// live place and condition (living 0, dead `0xA00`).
///
/// What follows the place is the live body's, as the exporter writes it
/// ([`live_state`]): a body that died since the load writes its live pose
/// tree; a living character an AI drives writes its AI home (an awake
/// guardian's), its live pose tree and its motion state. One stored dead
/// keeps its whole placement [Exarust's: the original writes every dead
/// body's live pose tree; a level's corpse is registered asleep and stays
/// on its stored pose].
pub fn characters_after(
    stored: &[Placement],
    live: &[LiveCharacter],
    gone: impl Fn(u32) -> bool,
) -> (Vec<Placement>, CharacterCounts) {
    let mut counts = CharacterCounts::default();
    let mut used = vec![false; live.len()];
    let mut out = Vec::with_capacity(stored.len() + live.len());
    for placement in stored {
        let here = Vec3::from_array(placement.position);
        let nearest = live
            .iter()
            .enumerate()
            .filter(|&(i, c)| !used[i] && same_record(c.id, placement.id))
            .min_by(|(_, a), (_, b)| {
                let da = Vec3::from_array(a.position).distance_squared(here);
                let db = Vec3::from_array(b.position).distance_squared(here);
                da.total_cmp(&db)
            })
            .map(|(i, _)| i);
        let Some(i) = nearest else {
            if placement.id & WORLD_DB == 0 && gone(placement.id) {
                counts.removed += 1;
            } else {
                counts.unspawned += 1;
                out.push(placement.clone());
            }
            continue;
        };
        used[i] = true;
        counts.kept += 1;
        let character = &live[i];
        let mut placement = placement.clone();
        if placement.is_dead() {
            // A level's dead lie on their stored pose and do not walk.
            out.push(placement);
            continue;
        }
        let travel = (Vec3::from_array(character.position) - here)
            .abs()
            .max_element();
        let died = character.dead && !placement.is_dead();
        if travel > CHARACTER_UNMOVED || died {
            counts.moved += 1;
            placement.position = character.position;
            placement.yaw = character.yaw;
            if died {
                placement.condition = exa_formats::characters::DEAD;
            }
        }
        live_state(&mut placement, character);
        out.push(placement);
    }
    for (character, _) in live.iter().zip(&used).filter(|(_, used)| !**used) {
        counts.added += 1;
        let mut placement = Placement {
            id: character.id,
            position: character.position,
            yaw: character.yaw,
            condition: if character.dead {
                exa_formats::characters::DEAD
            } else {
                0
            },
            pose: SavedPose::None,
            motion: None,
            ai_home: None,
            powers: Vec::new(),
        };
        live_state(&mut placement, character);
        out.push(placement);
    }
    (out, counts)
}

/// The live body's chunks on a placement, as the exporter's character
/// section writes them after the place: a dead body its live pose tree
/// (`0xCAC0EB00`; a body with no physical parts keeps a stored tree, else
/// writes one record at its feet [Exarust's: the original's every
/// character has a body]); a living one an AI drives its AI home
/// (`0xCACE0F77`, only for an awake guardian), its live pose tree and its
/// motion state (`0xCAC0EA00`). A living character no AI drives keeps the
/// stored pose and motion. No character of the section writes its running
/// powers: the exporter's character walk has no running-powers chunk (only
/// the player's own placement in `#playerdata` writes one,
/// [`crate::power_save`]), so a scene character's powers end with the save,
/// and a stored chunk is dropped.
pub fn live_state(placement: &mut Placement, character: &LiveCharacter) {
    placement.powers.clear();
    if placement.is_dead() {
        if let Some(pose) = character.pose.clone().filter(|p| !p.is_empty()) {
            placement.pose = SavedPose::Tree(pose);
        } else if !matches!(placement.pose, SavedPose::Tree(_)) {
            let at = Affine3A::from_translation(Vec3::from_array(character.position));
            placement.pose = SavedPose::Tree(vec![PoseRecord {
                frame: exa_character::pose::frame_of_affine(&at),
                word: 1,
                children: 0,
            }]);
        }
        placement.motion = None;
        placement.ai_home = None;
        return;
    }
    let Some(motion) = character.motion else {
        return;
    };
    placement.ai_home = character.ai_home;
    // The motion state goes with the pose tree; a body without parts
    // writes neither.
    match character.pose.clone().filter(|p| !p.is_empty()) {
        Some(pose) => {
            placement.pose = SavedPose::Tree(pose);
            placement.motion = Some(motion);
        }
        None => placement.motion = None,
    }
}

/// The saved state a loaded character's placement carried (`0xCAC0EA00`,
/// `0xCACE0F77`), put on the spawned living body for
/// [`restore_saved_state`].
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SavedCharacterState {
    pub motion: Option<MotionState>,
    pub ai_home: Option<AiHome>,
}

/// Puts a loaded character's saved state back once, as the original's
/// character builder does after the body is made: the saved motion
/// ([`loaded_motion`]) starts with no blend and takes its saved progress
/// (without one the spawn's start motion stays); the AI home becomes the
/// guardian's home (cm) and heading, and its statue byte is cleared.
#[allow(clippy::type_complexity)]
pub fn restore_saved_state(
    mut commands: Commands,
    motions: Option<Res<exa_anim::Motions>>,
    mut loaded: Query<(
        Entity,
        &SavedCharacterState,
        Option<&mut exa_anim::Animator>,
        Option<&mut exa_ai::brain::Npc>,
        Option<&exa_character::record::CharacterData>,
    )>,
) {
    for (entity, saved, animator, npc, data) in &mut loaded {
        let who = data.map_or("?", |d| d.record.name.as_str());
        if let (Some(motion), Some(mut anim), Some(motions)) =
            (saved.motion, animator, motions.as_deref())
        {
            let motion = loaded_motion(motion);
            if let Some(action) = motions.action_name(motion.state).map(str::to_string) {
                let loops = motions.clip(&anim, &action).is_some_and(|c| c.loops());
                let then = (!loops).then(|| "Idle".to_string());
                anim.start(&action, loops, then, 0, exa_anim::Ease::Linear);
                anim.progress = motion.value;
                info!(
                    "load: {who} resumes motion {:#x} ({action}) at {:.3}",
                    motion.state, motion.value
                );
            }
        }
        if let (Some(home), Some(mut npc)) = (saved.ai_home, npc) {
            npc.guard_home = Some((Vec3::from_array(home.position) * 100.0, home.heading));
            npc.statue = false;
            info!("load: {who} takes its saved AI home {:?}", home.position);
        }
        commands.entity(entity).remove::<SavedCharacterState>();
    }
}

/// The character part of a story save's level section: the live scene's
/// characters, the level store's records that left it, the world records
/// the placements may name and the live local store's body.
#[derive(Debug, Clone)]
pub struct SectionCharacters<'a> {
    pub live: &'a [LiveCharacter],
    /// Low ids of the records the live level store holds; a stored
    /// placement whose record it lacks left the level.
    pub held: &'a [u32],
    pub world: &'a [exa_formats::characters::CharacterRecord],
    /// The local store's body (`None`: the store is empty and the section
    /// writes none).
    pub database: Option<Vec<u8>>,
}

/// The level section a save writes: `source` (the section the level was
/// read from) with the live placed objects and the live local tier
/// (`records`, the tier's writer, and `strings`, its library's) written
/// over it, exported with the save switches ([`export_level_scene`]).
#[allow(clippy::too_many_arguments)]
pub fn saved_section(
    source: &[u8],
    spawned: &[u32],
    live: &[LivePlacement],
    props: &[LiveProp],
    records: Vec<u8>,
    strings: Vec<u8>,
    story: bool,
    settings: SettingsState,
    characters: Option<SectionCharacters<'_>>,
) -> exa_formats::Result<(Vec<u8>, PlacementCounts, PropCounts)> {
    let mut scene = Scene::parse(source)?;
    let mut raw = LevelRaw::scan(source)?;
    if let Some(characters) = characters {
        let chars = scene.characters.get_or_insert_with(Default::default);
        let (placements, _) = characters_after(&chars.placements, characters.live, |id| {
            !characters.held.contains(&(id & ID_MASK))
        });
        chars.placements = placements;
        // The world records the placements name, for their node names.
        chars.records.extend(characters.world.iter().cloned());
        raw.character_database = characters.database;
    }
    let (placed, counts) = placed_after(
        &scene.placed_objects,
        scene.nodes.len(),
        scene.props.len(),
        spawned,
        live,
    );
    scene.placed_objects = placed;
    let prop_counts = props_after(&mut scene.props, props);
    raw.object_database = Some(records);
    raw.object_strings = Some(strings);
    let bytes = export_level_scene(&scene, &raw, story, settings)?;
    Ok((bytes, counts, prop_counts))
}

/// A placed object's entity as the save reads it: its id, its place in
/// the loaded list, its world transform and whether a body moves it.
type PlacedNode = (
    Entity,
    &'static mut exa_world::PlacedItem,
    Option<&'static exa_world::PlacedIndex>,
    &'static GlobalTransform,
    Has<exa_physics::prop::BodyNode>,
);

/// A prop's root entity as the save reads it: its place in the section's
/// list, its world transform and whether a body moves it directly.
type PropNode = (
    Entity,
    &'static exa_world::PropIndex,
    &'static GlobalTransform,
    Has<exa_physics::prop::BodyNode>,
);

/// What the save reads of the running level: where it came from and its
/// placed objects and props with their bodies.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LevelSaveScene<'w, 's> {
    source: Option<Res<'w, exa_world::LevelSource>>,
    placed: Query<'w, 's, PlacedNode>,
    props: Query<'w, 's, PropNode>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static exa_physics::prop::PropBody,
            &'static exa_physics::body::Ragdoll,
        ),
    >,
    parents: Query<'w, 's, &'static ChildOf>,
    characters: Query<'w, 's, SceneCharacter, Without<crate::player::PlayerBody>>,
    /// The scene characters' record ids by body, for the running Loyalty
    /// and Command Dead a save writes (their targets go by character id).
    records: Query<'w, 's, SceneRecord, SceneRecordFilter>,
    local: Option<ResMut<'w, crate::world_characters::LocalCharacters>>,
    motions: Option<Res<'w, exa_anim::Motions>>,
    /// The golem alarm the story's section carries in its settings chunk
    /// (the frame's copy).
    alarm: Option<Res<'w, crate::golem_alarm::GolemAlarm>>,
    /// The dead player's body the flow left in the level, and the player
    /// bodies' place, turn and parts ([`Self::left_character`]).
    left: Option<Res<'w, crate::left_behind::LeftBody>>,
    left_bodies: Query<'w, 's, LeftBodyParts, With<crate::player::PlayerBody>>,
    /// The owned nodes and mechanisms the part records and object words
    /// read.
    owned: OwnedParts<'w, 's>,
}

/// A mechanism body's glue, for its live object word.
type MechanismGlue = (
    Entity,
    Option<&'static exa_physics::body::DoorBody>,
    Option<&'static exa_physics::body::SlideBody>,
    Option<&'static exa_physics::body::LeverBody>,
    Option<&'static exa_physics::body::PlateBody>,
);

/// A cloth part as the frame holds it: its cloth (the start the frame
/// built it from), its file data, its drawn mesh and its world transform.
type ClothPart = (
    &'static exa_world::cloth::ClothInstance,
    &'static exa_world::cloth::ClothSource,
    &'static GlobalTransform,
);

/// What the save reads of the placed objects' and props' own nodes: their
/// model trees, the bodies that move them, the cloth on them and the
/// mechanisms' glue.
#[derive(bevy::ecs::system::SystemParam)]
pub struct OwnedParts<'w, 's> {
    trees: Query<'w, 's, &'static exa_world::saved_parts::PartTree>,
    body_nodes: Query<'w, 's, &'static exa_physics::prop::BodyNode>,
    ragdolls: Query<'w, 's, &'static exa_physics::body::Ragdoll>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    node_parts: Query<'w, 's, &'static exa_world::node_parts::NodeParts>,
    cloths: Query<'w, 's, ClothPart>,
    meshes: Option<Res<'w, Assets<Mesh>>>,
    mechanisms: Query<'w, 's, MechanismGlue, MechanismFilter>,
}

/// The bodies that carry a mechanism's glue.
type MechanismFilter = Or<(
    With<exa_physics::body::DoorBody>,
    With<exa_physics::body::SlideBody>,
    With<exa_physics::body::LeverBody>,
    With<exa_physics::body::PlateBody>,
)>;

/// A cloth particle further than this from its start (cm, any axis) makes
/// its cloth write its live particles [Exarust's: the original writes every
/// cloth's particles].
const CLOTH_UNMOVED_CM: f32 = 1.0;

impl OwnedParts<'_, '_> {
    /// A node's live world frame: its body part's for a node a body moves,
    /// else its world transform.
    fn frame(&self, entity: Entity) -> Option<Affine3A> {
        if let Ok(node) = self.body_nodes.get(entity)
            && let Ok(body) = self.ragdolls.get(node.body)
            && let Some(part) = body.parts.get(node.part)
        {
            return Some(part.transform());
        }
        self.globals.get(entity).ok().map(GlobalTransform::affine)
    }

    /// The live particles of a cloth node that moved off its start: every
    /// position element of its mesh in the level's frame (metres), from
    /// the drawn mesh of each of its cloth parts. `None` when it has not
    /// moved, or when its parts do not cover every element.
    fn cloth(&self, node: Entity) -> Option<Vec<[f32; 3]>> {
        let meshes = self.meshes.as_deref()?;
        let parts = self.node_parts.get(node).ok()?;
        let mut elements: Vec<Option<[f32; 3]>> = Vec::new();
        let mut moved = false;
        for &part in &parts.parts {
            let Ok((instance, source, global)) = self.cloths.get(part) else {
                continue;
            };
            let mesh = meshes.get(&instance.mesh)?;
            let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
            let part_elements = source.mesh.part_elements.get(source.part)?;
            if elements.is_empty() {
                elements = vec![None; source.mesh.elements];
            }
            for (v, at) in positions.iter().enumerate() {
                let at = Vec3::from_array(*at);
                let (Some(&element), Some(&particle)) =
                    (part_elements.get(v), instance.vertex_particle.get(v))
                else {
                    return None;
                };
                if let Some(start) = instance.start.get(particle) {
                    moved |= (at * 100.0 - *start).abs().max_element() > CLOTH_UNMOVED_CM;
                }
                let slot = elements.get_mut(element as usize)?;
                *slot = Some(global.affine().transform_point3(at).to_array());
            }
        }
        if !moved || elements.is_empty() {
            return None;
        }
        elements.into_iter().collect()
    }

    /// The model nodes of the object or prop rooted at `root` as they stand
    /// ([`part_records`] writes its records from them); `None` without a
    /// tree that can hold one.
    fn live_parts(&self, root: Entity) -> Option<Vec<LivePart>> {
        use exa_world::saved_parts::PartKind;
        let tree = self.trees.get(root).ok()?;
        let frames: Vec<Option<Affine3A>> =
            tree.nodes.iter().map(|n| self.frame(n.entity)).collect();
        let parts: Option<Vec<LivePart>> = tree
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let world = frames[i]?;
                let moved = match node.parent.and_then(|p| frames.get(p).copied().flatten()) {
                    Some(parent) => {
                        rows_apart(&rows_of(parent.inverse() * world), &rows_of(node.spawned))
                            > UNMOVED
                    }
                    None => false,
                };
                Some(LivePart {
                    name: node.name.clone(),
                    parent: node.parent,
                    kind: node.kind,
                    world,
                    moved,
                    cloth: (node.kind == PartKind::Cloth)
                        .then(|| self.cloth(node.entity))
                        .flatten(),
                })
            })
            .collect();
        parts
    }

    /// The live object word of each mechanism by the placed object it
    /// belongs to (`owner`: the nearest one at or above its body's root): a
    /// door's or sliding leaf's stored word and a lever's word whole, a push
    /// surface's state as the word's low byte.
    fn words(
        &self,
        owner: impl Fn(Entity) -> Option<Entity>,
    ) -> std::collections::HashMap<Entity, LiveWord> {
        let mut out = std::collections::HashMap::new();
        for (entity, door, slide, lever, plate) in &self.mechanisms {
            let Some(object) = owner(entity) else {
                continue;
            };
            let whole = |value: i32| LiveWord {
                keep: 0,
                value: value as u32,
            };
            let word = if let Some(door) = door {
                whole(door.door.stored())
            } else if let Some(slide) = slide {
                whole(slide.door.stored())
            } else if let Some(lever) = lever {
                whole(lever.lever.word)
            } else if let Some(plate) = plate {
                LiveWord {
                    keep: !0xFF,
                    value: u32::from(plate.plate.saved()),
                }
            } else {
                continue;
            };
            // A pair's leaves keep the one object word between them (their
            // orders share bit 0); the first leaf found writes it.
            out.entry(object).or_insert(word);
        }
        out
    }
}

/// A player body's place, locomotion turn and parts, for the left body's
/// placement.
type LeftBodyParts = (
    Entity,
    &'static GlobalTransform,
    Option<&'static exa_physics::body::locomotion::Locomotor>,
    Option<&'static exa_physics::body::Ragdoll>,
);

/// A scene character's body and record, for the save's target names.
type SceneRecord = (Entity, &'static exa_character::record::CharacterData);
/// The scene characters with a mind, the player left out.
type SceneRecordFilter = (
    With<crate::mind::casts::MindBody>,
    Without<crate::player::PlayerBody>,
);

/// A scene character as the save's flush reads it.
type SceneCharacter = (
    &'static GlobalTransform,
    &'static exa_character::record::CharacterData,
    Option<&'static exa_items::possession::Possessions>,
    Option<&'static exa_combat::Fighter>,
    Option<&'static exa_combat::StateFlags>,
    Option<&'static exa_combat::Condition>,
    Option<&'static crate::thaum::cast::PowerCaster>,
    Option<&'static exa_physics::body::locomotion::Locomotor>,
    Option<&'static exa_physics::body::Ragdoll>,
    Option<&'static exa_anim::Animator>,
    Option<&'static exa_ai::brain::Npc>,
    Option<&'static exa_ai::roles::Roles>,
    Option<&'static crate::mind::casts::MindBody>,
);

impl LevelSaveScene<'_, '_> {
    /// The flow's record of the dead player's body, if any.
    pub fn left(&self) -> Option<&crate::left_behind::LeftBody> {
        self.left.as_deref()
    }

    /// The dead player's body as one of the scene's characters, once its
    /// owned mark is cleared ([`crate::left_behind`]): record `id`, the
    /// body's place, its locomotion's heading, `dead`, and its live pose
    /// tree. `None` while the mark stands or without the body.
    pub fn left_character(&self, id: u32, dead: bool) -> Option<LiveCharacter> {
        let left = self.left.as_deref().filter(|left| left.exported)?;
        let (_, transform, locomotor, ragdoll) = self
            .left_bodies
            .iter()
            .find(|(entity, ..)| left.writes(*entity))?;
        let yaw = locomotor.map_or_else(
            || transform.rotation().to_euler(EulerRot::YXZ).0,
            |locomotor| locomotor.yaw,
        );
        let pose = ragdoll.map(|body| {
            let parts: Vec<_> = body
                .parts
                .iter()
                .map(|part| (part.bone, part.parent, part.transform()))
                .collect();
            live_pose_tree(&parts)
        });
        Some(LiveCharacter {
            id,
            position: transform.translation().to_array(),
            yaw,
            dead,
            pose,
            motion: None,
            ai_home: None,
        })
    }

    /// Each scene character's body id and record id
    /// ([`crate::power_save::record_ids`]).
    pub fn record_ids(&self) -> Vec<(crate::mind::casts::BodyId, u32)> {
        crate::power_save::record_ids(self.records.iter())
    }

    /// The arena save's entity rewrite: every scene object whose id is a
    /// local one the id remap table holds takes its world id
    /// ([`exa_formats::objdb::ObjectDb::remap_entity_id`]). Returns how many
    /// changed.
    pub fn rewrite_entity_ids(&mut self, db: &exa_formats::objdb::ObjectDb) -> usize {
        let mut changed = 0;
        for (_, mut item, ..) in &mut self.placed {
            let id = db.remap_entity_id(item.tagged_id);
            if id != item.tagged_id {
                let item = item.bypass_change_detection();
                item.tagged_id = id;
                item.id = id & exa_formats::objdb::ID_MASK;
                changed += 1;
            }
        }
        changed
    }

    /// The live placements: each placed object's id and condition, and its
    /// placement (an object a body moves directly is where its node is; one
    /// over a moved body takes the body's motion since it was registered).
    pub fn live_placements(&self) -> Vec<LivePlacement> {
        let motion = self.body_motion();
        let words = self.owned.words(|entity| self.owner(entity, false));
        let mut out: Vec<LivePlacement> = self
            .placed
            .iter()
            .map(|(entity, item, index, global, moved_directly)| {
                let affine = global.affine();
                let affine = match motion.get(&entity) {
                    Some(delta) if !moved_directly => *delta * affine,
                    _ => affine,
                };
                LivePlacement {
                    index: index.map(|i| i.0),
                    id: item.tagged_id,
                    condition: item.condition,
                    rows: rows_of(affine),
                    word: words.get(&entity).copied(),
                    parts: self.owned.live_parts(entity),
                }
            })
            .collect();
        // The list's order, then the drops in spawn order.
        out.sort_by_key(|p| p.index.unwrap_or(u32::MAX));
        out
    }

    /// The placed object (or, with `props`, the placed object or prop) at
    /// or above `entity`.
    fn owner(&self, entity: Entity, props: bool) -> Option<Entity> {
        let mut at = entity;
        loop {
            if self.placed.contains(at) || (props && self.props.contains(at)) {
                return Some(at);
            }
            at = self.parents.get(at).ok()?.parent();
        }
    }

    /// The motion of each free body since its registration, by the placed
    /// object or prop at or above its root node. A body hung from the world
    /// by its links (a door's leaf, a lever's arm, a chain) moves only its
    /// own nodes, never the object it hangs in, so it moves no placement.
    fn body_motion(&self) -> std::collections::HashMap<Entity, Affine3A> {
        let mut motion = std::collections::HashMap::new();
        for (entity, body, ragdoll) in &self.bodies {
            if !ragdoll.world.is_empty() {
                continue;
            }
            let (Some(placed), Some(part)) = (body.placed().first(), ragdoll.parts.first()) else {
                continue;
            };
            let delta = part.transform() * placed.inverse();
            let mut at = entity;
            loop {
                if self.placed.contains(at) || self.props.contains(at) {
                    motion.entry(at).or_insert(delta);
                    break;
                }
                match self.parents.get(at) {
                    Ok(parent) => at = parent.parent(),
                    Err(_) => break,
                }
            }
        }
        motion
    }

    /// The live props: each prop root's placement in its parent's frame,
    /// a prop over a moved free body taking the body's motion since it was
    /// registered (its parent prop's motion too), in list order.
    pub fn live_props(&self) -> Vec<LiveProp> {
        let motion = self.body_motion();
        let world = |entity: Entity| -> Option<Affine3A> {
            let affine = self.globals.get(entity).ok()?.affine();
            let moved_directly = self.props.get(entity).is_ok_and(|(.., directly)| directly);
            Some(match motion.get(&entity) {
                Some(delta) if !moved_directly => *delta * affine,
                _ => affine,
            })
        };
        let mut out: Vec<LiveProp> = self
            .props
            .iter()
            .filter_map(|(entity, index, ..)| {
                let at = world(entity)?;
                let parent = self
                    .parents
                    .get(entity)
                    .ok()
                    .and_then(|p| world(p.parent()))
                    .unwrap_or(Affine3A::IDENTITY);
                Some(LiveProp {
                    index: index.0,
                    rows: rows_of(parent.inverse() * at),
                    parts: self.owned.live_parts(entity),
                })
            })
            .collect();
        out.sort_by_key(|p| p.index);
        out
    }

    /// The save's flush of the scene's characters (the player aside, whose
    /// record the save writes itself): each one's live possessions and
    /// health go into its record ([`crate::save_live::scene_character_record`]),
    /// which becomes the live copy in its store: the world store for a
    /// world record (a follower that took a door), the level's for the
    /// rest. Returns where each stands, for the level section.
    pub fn flush_characters(
        &mut self,
        mut world: Option<&mut crate::world_characters::WorldCharacters>,
    ) -> Vec<LiveCharacter> {
        let mut out = Vec::new();
        let (mut to_world, mut to_local) = (0, 0);
        let (mut posed, mut moving, mut homes, mut powered) = (0, 0, 0, 0);
        for (
            transform,
            data,
            possessions,
            fighter,
            flags,
            condition,
            caster,
            locomotor,
            ragdoll,
            animator,
            npc,
            roles,
            mind,
        ) in &self.characters
        {
            let record = crate::save_live::scene_character_record(
                data,
                possessions,
                fighter.map(|f| (f, flags.copied(), condition.copied(), caster)),
                roles,
            );
            if record.id & WORLD_DB != 0 {
                if let Some(world) = world.as_deref_mut() {
                    world.store(&record);
                    to_world += 1;
                }
            } else if let Some(local) = self.local.as_deref_mut()
                && local.store(&record)
            {
                to_local += 1;
            }
            // The body's yaw is its locomotion's; the node keeps its spawn
            // turn.
            let yaw = locomotor.map_or_else(
                || transform.rotation().to_euler(EulerRot::YXZ).0,
                |locomotor| locomotor.yaw,
            );
            let dead = condition.is_some_and(|c| c.is_dead());
            let pose = ragdoll.map(|body| {
                let parts: Vec<_> = body
                    .parts
                    .iter()
                    .map(|part| (part.bone, part.parent, part.transform()))
                    .collect();
                live_pose_tree(&parts)
            });
            // The AI's attached mode: a living character with a brain.
            let motion = npc.filter(|_| !dead).and_then(|_| {
                let animator = animator?;
                let state = self.motions.as_deref()?.action_index(&animator.action)?;
                Some(MotionState {
                    state,
                    value: animator.progress,
                })
            });
            // An awake guardian's home.
            let ai_home = npc
                .filter(|npc| {
                    !dead && !npc.statue && data.record.profession() & 0xff00 == HOME_RACE_PAIR
                })
                .and_then(|npc| npc.guard_home)
                .map(|(at, heading)| AiHome {
                    position: (at / 100.0).to_array(),
                    heading,
                });
            // The level walk writes no running powers; the count is for
            // the log.
            powered += usize::from(
                !dead
                    && (mind.is_some_and(|m| !m.powers.is_empty())
                        || caster.is_some_and(|c| !c.active.is_empty())),
            );
            posed += usize::from(pose.as_ref().is_some_and(|p| !p.is_empty()));
            moving += usize::from(motion.is_some());
            homes += usize::from(ai_home.is_some());
            out.push(LiveCharacter {
                id: record.id,
                position: transform.translation().to_array(),
                yaw,
                dead,
                pose,
                motion,
                ai_home,
            });
        }
        info!(
            "save: scene character state: {posed} live pose trees, {moving} motion states, {homes} AI homes, {powered} running powers the level walk does not write, {} dead",
            out.iter().filter(|c| c.dead).count()
        );
        // The scene's order is the entities'; keep it stable.
        info!(
            "save: {} scene characters flushed ({to_world} into the world character database, {to_local} into the level's)",
            out.len()
        );
        out
    }

    /// The section the save writes for the running level, named by its
    /// locale; `None` without a loaded level or when it does not export.
    /// A story save passes the scene's characters (`characters`, from
    /// [`Self::flush_characters`]) and the world records their placements
    /// may name (`world`): the section's placements and its character
    /// database are then the live ones ([`characters_after`],
    /// [`crate::world_characters::LocalCharacters::section_body`]).
    pub fn section(
        &mut self,
        db: &exa_formats::objdb::ObjectDb,
        story: bool,
        characters: Option<(
            &[LiveCharacter],
            &[exa_formats::characters::CharacterRecord],
        )>,
    ) -> Option<(String, Vec<u8>)> {
        let live = self.live_placements();
        let words = live.iter().filter(|p| p.word.is_some()).count();
        let props = self.live_props();
        let characters = characters.filter(|_| story).and_then(|(live, world)| {
            let local = self.local.as_deref_mut()?;
            let held: Vec<u32> = local.db.ids().collect();
            Some((live, world, held, local.section_body()))
        });
        let source = self.source.as_deref()?;
        let records = db.write(exa_formats::objdb::Tier::Local);
        let strings = db.library(exa_formats::objdb::Tier::Local).write();
        if let Some((live, _, held, _)) = &characters {
            let stored = exa_formats::rfc::Scene::parse(&source.bytes)
                .ok()
                .and_then(|scene| scene.characters)
                .map(|chars| chars.placements)
                .unwrap_or_default();
            let (after, counts) =
                characters_after(&stored, live, |id| !held.contains(&(id & ID_MASK)));
            // The first moved characters' stored and written places, for a
            // check of the live positions against the placements.
            for (before, now) in stored
                .iter()
                .zip(&after)
                .filter(|(a, b)| a.id == b.id && a.position != b.position)
                .take(4)
            {
                info!(
                    "save: character {:#x} written at {:?} (stored {:?}, yaw {:.3} -> {:.3})",
                    now.id, now.position, before.position, before.yaw, now.yaw
                );
            }
            info!(
                "save: level section {} characters: {} kept ({} moved), {} gone, {} never spawned, {} added",
                source.locale(),
                counts.kept,
                counts.moved,
                counts.removed,
                counts.unspawned,
                counts.added
            );
        }
        match saved_section(
            &source.bytes,
            &source.spawned,
            &live,
            &props,
            records,
            strings,
            story,
            self.alarm
                .as_deref()
                .map_or_else(SettingsState::reset, |alarm| alarm.saved_settings()),
            characters
                .as_ref()
                .map(|(live, world, held, database)| SectionCharacters {
                    live,
                    held,
                    world,
                    database: database.clone(),
                }),
        ) {
            Ok((bytes, counts, prop_counts)) => {
                info!(
                    "save: level section {} props: {} kept, {} moved ({} by more than 1 cm), {} with live part records",
                    source.locale(),
                    prop_counts.kept,
                    prop_counts.moved,
                    prop_counts.moved_far,
                    prop_counts.parts
                );
                info!(
                    "save: level section {} exported ({} bytes, local tier {} records): {} placed objects kept ({} moved, {} by more than 1 cm, {} with live part records, {} mechanism words), {} gone, {} never spawned, {} added",
                    source.locale(),
                    bytes.len(),
                    db.store(exa_formats::objdb::Tier::Local).len(),
                    counts.kept,
                    counts.moved,
                    counts.moved_far,
                    counts.parts,
                    words,
                    counts.removed,
                    counts.unspawned,
                    counts.added
                );
                Some((source.locale().to_string(), bytes))
            }
            Err(error) => {
                warn!(
                    "save: level section {} does not export: {error}",
                    source.locale()
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(id: u32, slot: i32, x: f32) -> PlacedObject {
        let mut object = exa_world::dropped_object(id, 0x0005_0000);
        object.slot = slot;
        object.transform[3] = [x, 0.0, 0.0];
        object
    }

    fn live(index: Option<u32>, id: u32, x: f32) -> LivePlacement {
        let mut rows = object(id, 0, x).transform;
        rows[3] = [x, 0.0, 0.0];
        LivePlacement {
            index,
            id,
            condition: 5,
            rows,
            word: None,
            parts: None,
        }
    }

    /// A spawned object that is gone leaves with what it held, the later
    /// objects' container numbers close up, an unspawned one stays, a moved
    /// one takes its live placement and a drop is added at the end.
    #[test]
    fn the_live_scene_rewrites_the_placed_objects() {
        // Two nodes and one prop: objects are numbered from 3.
        let placed = [
            object(0x8000_0001, 0, 1.0), // 0: gone
            object(0x8000_0002, 3, 2.0), // 1: inside 0, gone with it
            object(0x8000_0003, 0, 3.0), // 2: moved
            object(0x8000_0004, 5, 4.0), // 3: inside 2, unspawned
            object(0x8000_0005, 0, 5.0), // 4: kept in place
        ];
        let spawned = [0, 1, 2, 4];
        let scene = [
            live(Some(2), 0x8000_0003, 3.5),
            live(Some(4), 0x8000_0005, 5.0),
            live(None, 0x8000_000B, 9.0),
        ];
        let (out, counts) = placed_after(&placed, 2, 1, &spawned, &scene);
        assert_eq!(
            counts,
            PlacementCounts {
                kept: 2,
                moved: 1,
                moved_far: 1,
                removed: 2,
                unspawned: 1,
                added: 1,
                parts: 0,
            }
        );
        let ids: Vec<u32> = out.iter().map(|o| o.id).collect();
        assert_eq!(ids, [0x8000_0003, 0x8000_0004, 0x8000_0005, 0x8000_000B]);
        assert_eq!(out[0].transform[3], [3.5, 0.0, 0.0]);
        // The unspawned child now names its container's new number.
        assert_eq!(out[1].slot, 3);
        assert_eq!(out[2].transform[3], [5.0, 0.0, 0.0]);
        assert_eq!((out[3].slot, out[3].flags), (0, 0x0005_0000));
    }

    /// An object nothing moved keeps its stored rows exactly.
    #[test]
    fn an_unmoved_object_keeps_its_stored_rows() {
        let mut stored = object(7, 0, 1.0);
        stored.transform[0] = [0.6, 0.0, 0.8];
        let mut nearly = live(Some(0), 7, 1.0);
        nearly.rows = stored.transform;
        nearly.rows[3][1] = 3e-4;
        let (out, counts) = placed_after(&[stored.clone()], 0, 0, &[0], &[nearly]);
        assert_eq!(counts.moved, 0);
        assert_eq!(out[0].transform, stored.transform);
    }

    /// A shipped level's section written for a save reads back with the
    /// live local tier in place of its own, without the object the player
    /// took, with a moved object where it stands and a dropped one at the
    /// end (skipped without the game).
    #[test]
    fn a_shipped_level_saves_its_live_state_and_reads_it_back() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let source = pack.get("exanima01.rfc").unwrap();
        let scene = Scene::parse(source).unwrap();
        let count = scene.placed_objects.len() as u32;
        assert!(count > 2);
        // The first object (a world one, inside nothing) is taken; the
        // second moves 50 cm up; the rest stand.
        let taken = (0..count)
            .find(|&i| scene.placed_objects[i as usize].slot == 0)
            .unwrap();
        let spawned: Vec<u32> = (0..count).collect();
        let mut live: Vec<LivePlacement> = scene
            .placed_objects
            .iter()
            .enumerate()
            .filter(|&(i, object)| i as u32 != taken && object.slot == 0)
            .map(|(i, object)| LivePlacement {
                index: Some(i as u32),
                id: object.id,
                condition: ((object.flags >> 16) & 0xFF) as u8,
                rows: object.transform,
                word: None,
                parts: None,
            })
            .collect();
        let moved = live[0].index.unwrap();
        live[0].rows[3][1] += 0.5;
        live.push(LivePlacement {
            index: None,
            id: 0x8000_0003,
            condition: 0x40,
            rows: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 2.0, 3.0],
            ],
            word: None,
            parts: None,
        });
        // The live local tier: the level's own with one record dropped.
        let mut local = scene.local_database.clone().unwrap();
        let last = local.entries().last().unwrap().id;
        let at = local.find_index(last).unwrap();
        local.store(at, &[]);
        let records = local.write();
        let strings = scene.local_strings.clone().unwrap_or_default().write();
        let (saved, counts, _) = saved_section(
            source,
            &spawned,
            &live,
            &[],
            records.clone(),
            strings,
            true,
            SettingsState::reset(),
            None,
        )
        .unwrap();
        assert_eq!(counts.added, 1);
        assert!(counts.removed >= 1);
        let back = Scene::parse(&saved).unwrap();
        assert!(back.level_errors.is_empty(), "{:?}", back.level_errors);
        // The save exporter's option word 4: no mesh keeps its triangle
        // edge or flag sub-chunks.
        assert!(scene.nodes.iter().any(|node| matches!(
            &node.body,
            exa_formats::rfc::NodeBody::Mesh(m) if m.triangle_flags.is_some()
        )));
        assert!(back.nodes.iter().all(|node| match &node.body {
            exa_formats::rfc::NodeBody::Mesh(m) =>
                m.triangle_flags.is_none() && m.triangle_edges.is_none(),
            _ => true,
        }));
        assert!(saved.len() < source.len());
        assert_eq!(back.local_database.unwrap().write(), records);
        let kept = back.placed_objects.len();
        assert_eq!(kept, scene.placed_objects.len() - counts.removed + 1);
        let dropped = back.placed_objects.last().unwrap();
        assert_eq!(dropped.id, 0x8000_0003);
        assert!((dropped.transform[3][2] - 3.0).abs() < 1e-4);
        assert_eq!((dropped.flags >> 16) & 0xFF, 0x40);
        let moved_before = &scene.placed_objects[moved as usize];
        assert!(
            back.placed_objects
                .iter()
                .any(|object| object.id == moved_before.id
                    && (object.transform[3][1] - moved_before.transform[3][1] - 0.5).abs() < 1e-3)
        );
    }

    /// A prop the player pushed is saved where it stands: the prop pass
    /// writes the live placement over the stored one, keeps an unmoved
    /// prop's record as it was, and leaves an index past the list alone.
    #[test]
    fn the_prop_pass_writes_the_moved_props() {
        let prop = |x: f32| exa_formats::level::Prop {
            object: "barrel".into(),
            flags: 0,
            transform: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [x, 0.0, 0.0],
            ],
            parent: None,
            extra: 0,
            attachments: Vec::new(),
        };
        let mut props = vec![prop(1.0), prop(2.0)];
        let mut moved = props[1].transform;
        moved[3][2] += 0.4;
        let mut noise = props[0].transform;
        noise[3][1] += 1e-4;
        let counts = props_after(
            &mut props,
            &[
                LiveProp {
                    index: 0,
                    rows: noise,
                    parts: None,
                },
                LiveProp {
                    index: 1,
                    rows: moved,
                    parts: None,
                },
                LiveProp {
                    index: 7,
                    rows: moved,
                    parts: None,
                },
            ],
        );
        assert_eq!(
            counts,
            PropCounts {
                kept: 2,
                moved: 1,
                moved_far: 1,
                parts: 0,
            }
        );
        assert_eq!(props[0].transform, prop(1.0).transform);
        assert_eq!(props[1].transform, moved);
    }

    /// The running save reads the props from the scene: a prop root moved
    /// 40 cm is written at its new place in the level's frame, and a prop
    /// placed under it keeps its world place, so its record (in its
    /// parent's frame) changes the other way. The written section reads
    /// back with both (skipped without the game).
    #[test]
    fn the_running_save_writes_the_live_props() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let source = pack.get("exanima01.rfc").unwrap();
        let scene = Scene::parse(source).unwrap();
        let top = scene.props.iter().position(|p| p.parent.is_none()).unwrap();
        #[derive(Resource, Default)]
        struct Out(Vec<LiveProp>, Option<(String, Vec<u8>)>);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Out>()
            .insert_resource(exa_world::LevelSource {
                entry: "exanima01.rfc".into(),
                bytes: source.into(),
                from_save: false,
                spawned: Vec::new(),
            })
            .add_systems(Update, |mut scene: LevelSaveScene, mut out: ResMut<Out>| {
                out.0 = scene.live_props();
                out.1 = scene.section(&exa_formats::objdb::ObjectDb::default(), false, None);
            });
        let root = app
            .world_mut()
            .spawn((Transform::default(), GlobalTransform::default()))
            .id();
        let rows = scene.props[top].transform;
        let stored = Transform::from_matrix(Mat4::from(Affine3A::from_cols(
            Vec3::from(rows[0]).into(),
            Vec3::from(rows[1]).into(),
            Vec3::from(rows[2]).into(),
            Vec3::from(rows[3]).into(),
        )));
        let moved = Transform::from_translation(stored.translation + Vec3::Z * 0.4)
            .with_rotation(stored.rotation);
        let parent = app
            .world_mut()
            .spawn((
                moved,
                GlobalTransform::from(moved),
                exa_world::PropIndex(top as u32),
                ChildOf(root),
            ))
            .id();
        // A second prop record (the next one) hung under the moved prop at
        // the world place it was stored at.
        let child_index = (top + 1) as u32;
        let child_world = Transform::from_xyz(3.0, 1.0, 2.0);
        app.world_mut().spawn((
            Transform::from_matrix(moved.to_matrix().inverse() * child_world.to_matrix()),
            GlobalTransform::from(child_world),
            exa_world::PropIndex(child_index),
            ChildOf(parent),
        ));
        app.update();
        let out = app.world().resource::<Out>();
        assert_eq!(out.0.len(), 2);
        let written = out.0[0].rows;
        assert!((Vec3::from(written[3]) - moved.translation).length() < 1e-4);
        let (locale, bytes) = out.1.clone().unwrap();
        assert_eq!(locale, "exanima01");
        let back = Scene::parse(&bytes).unwrap();
        assert_eq!(back.props.len(), scene.props.len());
        let at = Vec3::from(back.props[top].transform[3]);
        assert!(
            (at - Vec3::from(scene.props[top].transform[3]) - Vec3::Z * 0.4).length() < 1e-3,
            "{at}"
        );
        // The child's record is its place in the moved parent's frame.
        let child = Vec3::from(back.props[child_index as usize].transform[3]);
        let expected = moved
            .to_matrix()
            .inverse()
            .transform_point3(child_world.translation);
        assert!((child - expected).length() < 1e-3, "{child} {expected}");
        // Every other prop keeps its record.
        for (i, (a, b)) in scene.props.iter().zip(&back.props).enumerate() {
            if i != top && i != child_index as usize {
                assert!(rows_apart(&a.transform, &b.transform) < 1e-5, "{i}");
            }
        }
    }

    /// Leaving the triangle flags out of a save's section costs no cloth its
    /// pins: no shipped level stores a cloth mesh among its own nodes (the
    /// banners are objects from the libraries, whose meshes a load reads
    /// from the library, not from the section). Skipped without the game.
    #[test]
    fn no_shipped_level_keeps_a_cloth_in_its_own_nodes() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let mut levels = 0;
        for entry in pack.entries().iter().filter(|e| e.name.ends_with(".rfc")) {
            let name = &entry.name;
            let Ok(scene) = Scene::parse(pack.bytes(entry)) else {
                continue;
            };
            if scene.grid.is_none() && scene.placed_objects.is_empty() {
                continue;
            }
            levels += 1;
            let cloth: Vec<&str> = scene
                .nodes
                .iter()
                .filter(|node| node.cloth_marker)
                .map(|node| node.name.as_str())
                .collect();
            assert!(cloth.is_empty(), "{name}: {cloth:?}");
        }
        assert!(levels >= 4, "{levels}");
    }

    fn stored(id: u32, x: f32) -> Placement {
        Placement {
            id,
            position: [x, 0.0, 0.0],
            yaw: 0.5,
            condition: 0,
            pose: exa_formats::characters::SavedPose::None,
            motion: None,
            ai_home: None,
            powers: Vec::new(),
        }
    }

    fn standing(id: u32, x: f32, dead: bool) -> LiveCharacter {
        LiveCharacter {
            id,
            position: [x, 0.0, 0.0],
            yaw: 1.0,
            dead,
            pose: None,
            motion: None,
            ai_home: None,
        }
    }

    /// A two-part body: the pelvis at `x` and a thigh 10 cm below it.
    fn two_parts(x: f32) -> Vec<PoseRecord> {
        let pelvis = Affine3A::from_translation(Vec3::new(x, 1.0, 0.0));
        let thigh = Affine3A::from_translation(Vec3::new(x, 0.9, 0.0));
        live_pose_tree(&[(2, Some(1), thigh), (1, None, pelvis)])
    }

    /// The live pose tree starts at the pelvis and lists each child part
    /// after its parent with its bone number and child count.
    #[test]
    fn the_live_pose_tree_walks_the_parts_from_the_pelvis() {
        let tree = two_parts(3.0);
        assert_eq!(tree.len(), 2);
        assert_eq!((tree[0].word, tree[0].children), (1, 1));
        assert_eq!((tree[1].word, tree[1].children), (2, 0));
        assert_eq!(tree[0].frame[3], [3.0, 1.0, 0.0]);
        assert_eq!(tree[1].frame[3], [3.0, 0.9, 0.0]);
        let turned = Affine3A::from_rotation_y(0.7);
        let back = exa_character::pose::affine(&exa_character::pose::frame_of_affine(&turned));
        assert!(back.abs_diff_eq(turned, 1e-6));
        assert!(live_pose_tree(&[]).is_empty());
    }

    /// A death since the load writes the body's live pose tree; a living
    /// character an AI drives writes its pose, motion state and home; one
    /// without a body writes no motion; a corpse with no parts gets one
    /// record at its feet so the section still exports.
    #[test]
    fn the_live_body_state_goes_with_the_placement() {
        let mut died = standing(8, 5.0, true);
        died.pose = Some(two_parts(5.0));
        let mut walker = standing(4, 1.0, false);
        walker.pose = Some(two_parts(1.0));
        walker.motion = Some(MotionState {
            state: 0x13,
            value: 0.25,
        });
        walker.ai_home = Some(AiHome {
            position: [1.0, 0.0, 2.0],
            heading: 0.5,
        });
        let mut bodiless = standing(5, 2.0, false);
        bodiless.motion = walker.motion;
        let (out, _) = characters_after(
            &[stored(8, 5.0), stored(4, 1.0), stored(5, 2.0)],
            &[died, walker, bodiless, standing(9, 7.0, true)],
            |_| false,
        );
        assert!(out[0].is_dead());
        assert!(matches!(&out[0].pose, SavedPose::Tree(t) if t.len() == 2));
        assert!(matches!(&out[1].pose, SavedPose::Tree(t) if t.len() == 2));
        assert_eq!(out[1].motion.map(|m| m.state), Some(0x13));
        assert_eq!(out[1].ai_home.map(|h| h.heading), Some(0.5));
        assert_eq!(out[2].motion, None);
        assert!(matches!(&out[3].pose, SavedPose::Tree(t)
            if t.len() == 1 && t[0].frame[3] == [7.0, 0.0, 0.0]));
    }

    /// An attack motion loads as the defence stance with no progress.
    #[test]
    fn a_loaded_attack_motion_becomes_the_defence_stance() {
        let attack = MotionState {
            state: 7,
            value: 0.4,
        };
        assert_eq!(
            loaded_motion(attack),
            MotionState {
                state: DEFENCE_MOTION,
                value: 0.0
            }
        );
        let walk = MotionState {
            state: 0x13,
            value: 0.4,
        };
        assert_eq!(loaded_motion(walk), walk);
    }

    /// An idling character keeps its placement, a moved one takes its live
    /// place, one who left the level's store leaves, one never spawned
    /// stays, a death is written and an arrival with a world id is added.
    #[test]
    fn the_live_scene_rewrites_the_character_placements() {
        let placements = [
            stored(4, 1.0), // idles
            stored(5, 2.0), // walks 3 m
            stored(6, 3.0), // left with the player
            stored(7, 4.0), // never built
            stored(8, 5.0), // died where it stood
            stored(2, 6.0), // a level record 2: not the world follower 2
        ];
        let live = [
            standing(4, 1.05, false),
            standing(5, 5.0, false),
            standing(8, 5.0, true),
            standing(2, 6.0, false),
            standing(WORLD_DB | 2, 9.0, false),
        ];
        let held = [4, 5, 7, 8, 2];
        let (out, counts) = characters_after(&placements, &live, |id| !held.contains(&id));
        // A stored corpse keeps its placement whatever its node says.
        let mut corpse = stored(9, 1.0);
        corpse.condition = exa_formats::characters::DEAD;
        let (kept, _) = characters_after(
            std::slice::from_ref(&corpse),
            &[standing(9, 4.0, true)],
            |_| false,
        );
        assert_eq!(kept, [corpse]);
        assert_eq!(
            counts,
            CharacterCounts {
                kept: 4,
                moved: 2,
                removed: 1,
                unspawned: 1,
                added: 1,
            }
        );
        let ids: Vec<u32> = out.iter().map(|p| p.id).collect();
        assert_eq!(ids, [4, 5, 7, 8, 2, WORLD_DB | 2]);
        assert_eq!((out[0].position[0], out[0].yaw), (1.0, 0.5));
        assert_eq!((out[1].position[0], out[1].yaw), (5.0, 1.0));
        assert!(out[3].is_dead());
        assert_eq!(out[4].position[0], 6.0);
        assert_eq!(out[5].position, [9.0, 0.0, 0.0]);
        assert_eq!(out[5].condition, 0);
    }

    /// Derrin carried into exanima02 under world id 2 is written into the
    /// saved section's placements beside the level's own characters, and
    /// the section placed again finds him in the world store at level 3
    /// (skipped without the game).
    #[test]
    fn a_carried_follower_is_written_into_the_saved_section() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let source = pack.get("exanima02.rfc").unwrap();
        let scene = Scene::parse(source).unwrap();
        let chars = scene.characters.clone().unwrap();
        let mut local = crate::world_characters::LocalCharacters::of_section(source);
        assert!(!local.db.is_empty());
        let mut derrin = exa_formats::characters::CharacterRecord::reset();
        derrin.id = WORLD_DB | 2;
        derrin.name = "Derrin".into();
        derrin.locale = 3;
        let live = [LiveCharacter {
            id: WORLD_DB | 2,
            position: [1.0, 0.5, 2.0],
            yaw: 0.25,
            dead: false,
            pose: None,
            motion: None,
            ai_home: None,
        }];
        let held: Vec<u32> = local.db.ids().collect();
        let records = scene
            .local_database
            .clone()
            .map(|db| db.write())
            .unwrap_or_default();
        let strings = scene.local_strings.clone().unwrap_or_default().write();
        let world = [derrin.clone()];
        let (saved, _, _) = saved_section(
            source,
            &[],
            &[],
            &[],
            records,
            strings,
            true,
            SettingsState::reset(),
            Some(SectionCharacters {
                live: &live,
                held: &held,
                world: &world,
                database: local.section_body(),
            }),
        )
        .unwrap();
        let back = Scene::parse(&saved).unwrap();
        let back_chars = back.characters.as_ref().unwrap();
        // Nobody of the level's stood in the scene, so all are kept.
        assert_eq!(back_chars.placements.len(), chars.placements.len() + 1);
        assert_eq!(back_chars.records.len(), chars.records.len());
        let last = back_chars.placements.last().unwrap();
        assert_eq!(last.id, WORLD_DB | 2);
        assert!((last.position[2] - 2.0).abs() < 1e-3);
        let placed = exa_world::placed_characters(&back, &[], Some(3), &world);
        assert!(placed.iter().any(|c| c.name == "Derrin"));
        // Elsewhere (level 2) the locale gate keeps him out.
        let elsewhere = exa_world::placed_characters(&back, &[], Some(2), &world);
        assert!(!elsewhere.iter().any(|c| c.name == "Derrin"));
    }

    /// A level character who died since the load, and one walking, are
    /// written into the saved section with their live pose trees (the
    /// walker with its motion state too) and read back so (skipped without
    /// the game). Without a pose the dead body's placement would not export.
    #[test]
    fn a_new_corpse_and_a_walker_save_their_live_bodies() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let source = pack.get("exanima02.rfc").unwrap();
        let scene = Scene::parse(source).unwrap();
        let chars = scene.characters.clone().unwrap();
        let living: Vec<&Placement> = chars.placements.iter().filter(|p| !p.is_dead()).collect();
        let (a, b) = (living[0], living[1]);
        let mut died = LiveCharacter {
            id: a.id,
            position: a.position,
            yaw: a.yaw,
            dead: true,
            pose: Some(two_parts(a.position[0])),
            motion: None,
            ai_home: None,
        };
        died.position[0] += 0.5;
        let walker = LiveCharacter {
            id: b.id,
            position: b.position,
            yaw: b.yaw,
            dead: false,
            pose: Some(two_parts(b.position[0])),
            motion: Some(MotionState {
                state: 0x13,
                value: 0.5,
            }),
            ai_home: None,
        };
        let mut local = crate::world_characters::LocalCharacters::of_section(source);
        let held: Vec<u32> = local.db.ids().collect();
        let records = scene
            .local_database
            .clone()
            .map(|db| db.write())
            .unwrap_or_default();
        let strings = scene.local_strings.clone().unwrap_or_default().write();
        let (saved, _, _) = saved_section(
            source,
            &[],
            &[],
            &[],
            records,
            strings,
            true,
            SettingsState::reset(),
            Some(SectionCharacters {
                live: &[died, walker],
                held: &held,
                world: &[],
                database: local.section_body(),
            }),
        )
        .unwrap();
        let back = Scene::parse(&saved).unwrap();
        let back = back.characters.unwrap();
        let corpse = back.placements.iter().find(|p| p.id == a.id).unwrap();
        assert!(corpse.is_dead());
        let SavedPose::Tree(tree) = &corpse.pose else {
            panic!("no pose tree");
        };
        assert_eq!(tree.len(), 2);
        assert!((tree[0].frame[3][1] - 1.0).abs() < 1e-4);
        let moving = back.placements.iter().find(|p| p.id == b.id).unwrap();
        assert_eq!(moving.motion.map(|m| m.state), Some(0x13));
        assert!(matches!(&moving.pose, SavedPose::Tree(t) if t.len() == 2));
    }

    #[test]
    fn a_world_transform_gives_its_rows() {
        let affine = Affine3A::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 2.0, 3.0),
        );
        let rows = rows_of(affine);
        let back = exa_assets::model::transform_from_rows(&rows);
        assert!(back.translation.abs_diff_eq(Vec3::new(1.0, 2.0, 3.0), 1e-6));
        let turned = back.rotation * Vec3::X;
        assert!(
            turned.abs_diff_eq(affine.matrix3 * Vec3::X, 1e-5),
            "{turned}"
        );
    }
}

#[cfg(test)]
#[path = "level_save_parts_test.rs"]
mod parts_test;
