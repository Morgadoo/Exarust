//! The conversation engine: which dialogue two characters can hold, how a
//! dialogue runs node by node, the reply panel's choices, the narrator.
//!
//! **Finding a dialogue.** Owner A, partner B. For each of A's roles R (in
//! list order): for each of B's roles Q, R's entry aimed at Q; then, for
//! each relation role A holds toward B, R's entry aimed at it; then R's
//! entry aimed at 0. After all of A's roles, every dialogue of each
//! relation role A holds toward B. The first that the caller's purpose
//! accepts wins; A→B is tried, then B→A. Topic memory travels with the
//! match: a role-to-role dialogue files it under (aimed-at role, R), a
//! relation role's dialogue under (the relation role, its aim). A live
//! conversation of the same graph and pair is reused.
//!
//! **Running a node.** Each actor's mood toward the other is a hostility
//! code (0x600 hostile or engaged with concern above 4, 0x200 above 1.2)
//! plus an attitude band (above 70: 0x10, above 30: 0x20, at least −30:
//! 0x40, below that: 0x80; the inner −60 test can never fail and its 0x100
//! band is unreachable). The attitude is first scaled by ((10 − x)/10)²
//! for a contact value x ≤ 10; past that (and for an unordered value) it
//! counts for nothing. Every test is the original's `<=`, so an unordered
//! attitude takes the > 70 band and an unordered concern the hostile code.
//! Branches of the current group are tried in order: a line of either
//! player slot becomes a reply choice when choices are being offered (after
//! the branch's condition passes); another speaker's line is a candidate if
//! the branch has no mood flags or the speaker's hostility code does not
//! exceed the branch's. The first branch with a candidate is spoken: one
//! line at random, `{…}` substituted, as a bubble; its action runs and its
//! replies become the current group. An exhausted group follows the
//! branch's link to another node, or the conversation ends and the node is
//! recorded as used.
//!
//! **Pacing.** A conversation steps when its next time comes: half the
//! bubble's duration after it is due. Waiting for a reply it ends after
//! 30 s. A character out of action ends its conversations.
//!
//! Evidence: the dialogue unit's search, open, start, node evaluator,
//! branch, step, end, event, topic, panel and narrator routines and their
//! constant tables (REA on `Exanima.exe` 0.9.5).

use exa_formats::roles::{Dialogue, NODE_ENTRY, NODE_NOT_ENTRY, Name, Node, RoleDb};

use crate::bubbles::{Bubbles, Speaker};
use crate::builtins::Host;
use crate::cast::{Cast, Character, Memory};
use crate::interp::{self, Context};
use crate::topics::{self, MemoryKey};

/// Arousal at or below which an AI greets (and nothing calmer is asked).
pub const GREET_AROUSAL: f32 = 1.0;
/// Arousal at or below which `StartDialogue` opens a conversation.
pub const START_AROUSAL: f32 = 10.0;
/// A conversation waiting for a reply ends after this long (ms).
pub const REPLY_WAIT: u32 = 30_000;

/// How calm the owner must be for a node of calm class k (node flag bits
/// 4–7): its arousal at or below entry k. The table is 16 words; past
/// entry 4 it runs into the zero padding, the event bits and the event
/// names that follow it, read as floats.
pub const CALM: [u32; 16] = [
    0x3F80_0000, // 1
    0x4120_0000, // 10
    0x41A0_0000, // 20
    0x4348_0000, // 200
    0x4348_0000, // 200
    0,
    0,
    0,
    1,
    2,
    4,
    8,
    0x7267_6741,
    0x6F73_7365,
    0x0000_0072,
    0,
];

/// Event nodes: the bit an AI event sets and the node title it runs.
pub const EVENTS: [(u32, &str); 4] = [
    (1, "Aggressor"),
    (2, "Engage"),
    (4, "Intimidated"),
    (8, "Threatened"),
];

/// A graph: a role entry's dialogue or the narrator's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphRef {
    Entry { role: u32, entry: usize },
    Narrator,
}

/// The dialogue data.
#[derive(Debug, Clone, Copy)]
pub struct Library<'a> {
    pub roles: &'a RoleDb,
    pub narrator: Option<&'a Dialogue>,
}

impl<'a> Library<'a> {
    pub fn graph(&self, g: GraphRef) -> Option<&'a Dialogue> {
        match g {
            GraphRef::Entry { role, entry } => {
                self.roles.get(role)?.entries.get(entry)?.dialogue.as_ref()
            }
            GraphRef::Narrator => self.narrator,
        }
    }

    /// `role`'s first entry aimed at `target` (no character condition), if
    /// it carries a dialogue.
    pub fn aimed(&self, role: u32, target: u32) -> Option<GraphRef> {
        let rec = self.roles.get(role)?;
        let entry = rec
            .entries
            .iter()
            .position(|e| e.target == target && e.condition == 0)?;
        rec.entries[entry].dialogue.as_ref()?;
        Some(GraphRef::Entry { role, entry })
    }
}

/// Where a conversation stands in its graph: a node and the group of
/// branches to choose from next (a path of branch indices from the
/// node's opening group).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// Node number (from 1).
    pub node: usize,
    pub path: Vec<usize>,
    /// A branch of this cursor was taken.
    pub visited: bool,
}

/// Which cursor of a conversation is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorSlot {
    /// Followed a link or was picked from the topic list.
    Linked,
    /// Started at an entry or named node.
    Started,
}

/// Conversation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    New,
    /// A line is up; the next step is due at `next_at`.
    Speaking,
    /// Waiting (for a reply, or idle).
    Waiting,
    Ended,
}

/// One running conversation.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub graph: GraphRef,
    /// Actor slots: owner, partner, and an event's third actor.
    pub actors: Vec<Option<Speaker>>,
    /// Whose relation record keeps the memory, and about whom.
    pub owner: Option<u32>,
    pub partner: Option<u32>,
    /// Memory filing: the block's key and its relation-role entry.
    pub key: u32,
    pub id: u32,
    /// The role whose dialogue this is (`Role.` in its scripts).
    pub role: u32,
    /// One word per node.
    pub words: Vec<u32>,
    /// The requester's arousal when opened (calm classes test it).
    pub arousal: f32,
    /// The requester's mood code when opened (stored; nothing reads it).
    pub mood_code: u32,
    pub state: State,
    pub linked: Option<Cursor>,
    pub started: Option<Cursor>,
    pub current: Option<CursorSlot>,
    /// The link a taken branch asked for (1-based, 0 none).
    pub jump: u32,
    /// Clock (ms) of the next step.
    pub next_at: u32,
    /// Choice counter while offering (topic number).
    pub counter: i32,
    /// The node picked from the topic list last.
    pub picked: usize,
    /// Mood mask per actor slot (1-based).
    pub moods: [u16; 4],
    pub refs: i32,
    pub ctx: Context<u32>,
}

impl Conversation {
    fn new(graph: GraphRef, actors: Vec<Option<Speaker>>) -> Self {
        Self {
            graph,
            actors,
            owner: None,
            partner: None,
            key: 0,
            id: 0,
            role: 0,
            words: Vec::new(),
            arousal: 0.0,
            mood_code: 0,
            state: State::New,
            linked: None,
            started: None,
            current: None,
            jump: 0,
            next_at: 0,
            counter: 0,
            picked: 0,
            moods: [0; 4],
            refs: 1,
            ctx: Context::new(Vec::new()),
        }
    }

    fn cursor(&self, slot: CursorSlot) -> Option<&Cursor> {
        match slot {
            CursorSlot::Linked => self.linked.as_ref(),
            CursorSlot::Started => self.started.as_ref(),
        }
    }

    fn cursor_mut(&mut self, slot: CursorSlot) -> Option<&mut Cursor> {
        match slot {
            CursorSlot::Linked => self.linked.as_mut(),
            CursorSlot::Started => self.started.as_mut(),
        }
    }

    pub fn current_cursor(&self) -> Option<&Cursor> {
        self.current.and_then(|s| self.cursor(s))
    }

    /// Bind the actors to the interpreter (character ids; the narrator's
    /// slot is empty).
    fn bind(&mut self) {
        let actors = self
            .actors
            .iter()
            .map(|a| match a {
                Some(Speaker::Character(id)) => Some(*id),
                _ => None,
            })
            .collect();
        let locals = std::mem::take(&mut self.ctx.locals);
        self.ctx = Context::new(actors);
        self.ctx.locals = locals;
        self.ctx.role = self.role;
        self.counter = 0;
    }

    fn memory_key(&self) -> Option<MemoryKey> {
        Some(MemoryKey {
            other: self.partner?,
            role: self.id,
            key: self.key,
        })
    }
}

/// The group a cursor points at.
fn group<'g>(graph: &'g Dialogue, cursor: &Cursor) -> &'g [exa_formats::roles::Branch] {
    let Some(node) = graph.node(cursor.node) else {
        return &[];
    };
    let mut g: &[exa_formats::roles::Branch] = &node.branches;
    for &i in &cursor.path {
        match g.get(i) {
            Some(b) => g = &b.replies,
            None => return &[],
        }
    }
    g
}

/// A reply-panel entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub text: Vec<u8>,
    /// 0: a reply in the running conversation; k ≥ 1: topic k of the
    /// list; 0xFFFF: `[Other]`.
    pub topic: u32,
    /// The branch it takes.
    pub branch: usize,
}

/// The reply panel's state (`TDialogueChoices`).
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub open: bool,
    pub partner: Option<Speaker>,
    pub entries: Vec<Choice>,
    /// The conversation whose replies are listed.
    pub conversation: Option<usize>,
    /// Topic dialogues kept for the list.
    pub conversations: Vec<usize>,
    /// Topics: (conversation, node title).
    pub topics: Vec<(usize, Name)>,
    /// `[Other]` toggled: topics shown instead of replies.
    pub other: bool,
    pub can_pick: bool,
    pub chosen: usize,
    /// The highlighted entry, from 1 (0: none).
    pub highlight: usize,
}

/// A pending narrator message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Narration {
    Topic(Name),
    Text(Vec<u8>),
}

/// What a script asked the game to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    GameEvent(u32),
    ShowInventory(u32),
    CheckpointSave,
    StartDialogue(Name),
    WitnessRole(u32),
    Report(String, String),
}

/// The world a step of the engine sees.
pub struct Scene<'a> {
    pub library: Library<'a>,
    pub cast: &'a mut Cast,
    /// Clock, ms.
    pub now: u32,
    /// World time, s.
    pub world_time: u32,
    /// The first player character (its lines are halved in length).
    pub player: Option<u32>,
    /// The second player slot of the original (its lines are reply choices
    /// too; only the first player's lines are shortened).
    pub player2: Option<u32>,
    pub locale: u32,
    pub effects: Vec<Effect>,
}

/// The engine's state: live conversations, bubbles, the panel.
#[derive(Debug, Clone, Default)]
pub struct Engine {
    /// Conversations by id; `None` once released.
    pub conversations: Vec<Option<Conversation>>,
    /// Live conversations, in start order (a reused one may appear twice).
    pub active: Vec<usize>,
    pub bubbles: Bubbles,
    pub panel: Panel,
    /// The random seed lines are drawn with (`seed × 0x8088405 + 1`).
    pub seed: u32,
    /// The conversation each character is in.
    pub current: Vec<(u32, usize)>,
    /// Pending narrator message and its due time.
    pub narration: Option<(u32, Narration)>,
    /// Player lines become choices while set.
    offering: bool,
    /// Set by a narrator start; with it set, narrator conversations end.
    pub narrator_done: bool,
    /// Characters whose conversations a script asked to leave.
    leaving: Vec<u32>,
    end_requested: bool,
    /// The conversation whose script last ran (`EndDialogue` closes it).
    last_run: Option<usize>,
    /// The AI event being dispatched: its bits and its subject.
    event_mask: u32,
    event_subject: Option<u32>,
}

/// Who asked, and what for.
#[derive(Debug, Clone)]
pub struct Request {
    pub a: u32,
    pub b: u32,
    /// The mood code of the requester toward the other.
    pub mood: u32,
    /// Searching B→A: the owner is `b`.
    pub swapped: bool,
    pub purpose: Purpose,
}

/// What to do once a dialogue is found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// An AI greeting: entry nodes of calm class 0.
    Greet,
    /// `StartDialogue`: the named node, or an entry node calm enough.
    Start(Name),
    /// AI events then a calm-enough entry node.
    Events,
    /// AI events only.
    EventsOnly,
    /// A calm-enough entry node.
    Reply,
    /// Run the named event node.
    Named(Name),
    /// Collect topics for the panel.
    Topics,
}

/// The mood mask of `a` toward `b` (hostility code + attitude band).
///
/// The comparisons are the original's `<=` tests, in the original's order:
/// an unordered value (NaN) takes the first branch of every test (hostile
/// contact, no attitude scaling and the > 70 band).
pub fn mood(cast: &Cast, a: u32, b: u32) -> u16 {
    let Some(ca) = cast.get(a) else { return 0 };
    let contact = ca.contact(b);
    let mut att = contact.map_or(0.0, |c| c.attitude);
    let mut m = 0;
    if let Some(c) = contact {
        if c.state & 2 != 0 {
            m = 0x600;
        } else if c.state == 1 {
            if c.concern <= 4.0 {
                if c.concern > 1.2 {
                    m = 0x200;
                }
            } else {
                m = 0x600;
            }
        }
        // Past 10 the attitude counts for nothing; otherwise the contact
        // value scales it by ((10 − x) / 10)².
        att = if c.range <= 10.0 {
            let s = f64::from(10.0 - c.range) * 0.1;
            (s * s * f64::from(att)) as f32
        } else {
            0.0
        };
    }
    m | if att <= 70.0 {
        if att <= 30.0 {
            if -30.0 <= att {
                if -60.0 <= att { 0x40 } else { 0x100 }
            } else {
                0x80
            }
        } else {
            0x20
        }
    } else {
        0x10
    }
}

/// The mood code a request carries (stored with the conversation).
pub fn mood_code(cast: &Cast, a: u32, b: u32) -> u32 {
    let Some(ca) = cast.get(a) else { return 0 };
    let contact = ca.contact(b);
    if let Some(c) = contact {
        if c.state == 1 {
            return 0x6000_0000;
        }
        if c.state & 2 != 0 {
            return 0x7000_0000;
        }
    }
    let att = contact.map_or(0.0, |c| c.attitude);
    if att <= 70.0 {
        if att <= 30.0 {
            if -30.0 <= att {
                if -60.0 <= att {
                    0x3000_0000
                } else {
                    0x5000_0000
                }
            } else {
                0x4000_0000
            }
        } else {
            0x2000_0000
        }
    } else {
        0x1000_0000
    }
}

/// Whether a speaker's lines are a player's: the original tests the
/// first player and the second player slot, and both become reply choices.
pub fn is_player(scene: &Scene<'_>, who: Speaker) -> bool {
    matches!(who, Speaker::Character(c) if Some(c) == scene.player || Some(c) == scene.player2)
}

/// Whether a node passes the greeting filter: calm class 0.
fn calm_any(node: &Node) -> bool {
    (node.flags & 0xF0) >> 4 == 0
}

/// Whether a node passes the calm filter: a class from 1 whose threshold
/// the stored arousal does not exceed.
fn calm_enough(node: &Node, arousal: f32) -> bool {
    let class = ((node.flags & 0xF0) >> 4) as usize;
    if class == 0 {
        return false;
    }
    let limit = f32::from_bits(CALM[class]);
    // An unordered arousal (NaN) passes, as the original's compare does.
    arousal.partial_cmp(&limit) != Some(std::cmp::Ordering::Greater)
}

/// The script host of a step: the scene plus the running conversation.
struct SceneHost<'s, 'a> {
    scene: &'s mut Scene<'a>,
    conv: Option<&'s mut Conversation>,
    leaving: &'s mut Vec<u32>,
    end_requested: &'s mut bool,
    /// `StartDialogue` requests, run once the script returns.
    start_requests: &'s mut Vec<(Option<u32>, Option<u32>, Name)>,
}

impl SceneHost<'_, '_> {
    fn character(&self, id: u32) -> Option<&Character> {
        self.scene.cast.get(id)
    }

    fn character_mut(&mut self, id: u32) -> Option<&mut Character> {
        self.scene.cast.get_mut(id)
    }

    fn graph(&self) -> Option<&Dialogue> {
        let g = self.conv.as_ref()?.graph;
        self.scene.library.graph(g)
    }
}

impl Host for SceneHost<'_, '_> {
    type Actor = u32;

    fn report(&mut self, title: &str, message: &str) {
        self.scene
            .effects
            .push(Effect::Report(title.to_string(), message.to_string()));
    }
    fn holds_role(&self, actor: u32, role: u32) -> bool {
        self.character(actor).is_some_and(|c| c.holds(role))
    }
    fn role_variables(&self, role: u32) -> Vec<Name> {
        self.scene
            .library
            .roles
            .get(role)
            .map(|r| r.variables.iter().map(|v| v.name).collect())
            .unwrap_or_default()
    }
    fn role_values(&self, actor: u32, role: u32) -> Option<Vec<i32>> {
        let c = self.character(actor)?;
        c.roles
            .iter()
            .find(|r| r.id == role)
            .map(|r| r.values.iter().map(|&v| v as i32).collect())
    }
    fn set_role_values(&mut self, actor: u32, role: u32, values: &[i32]) {
        if let Some(c) = self.character_mut(actor) {
            if let Some(r) = c.roles.iter_mut().find(|r| r.id == role) {
                r.values = values.iter().map(|&v| v as u32).collect();
            }
            c.roles_changed = true;
        }
    }
    fn add_role(&mut self, actor: u32, role: u32) {
        let db = self.scene.library.roles;
        if let Some(c) = self.character_mut(actor) {
            c.add_role(db, role);
            c.roles_changed = true;
        }
    }
    fn remove_role(&mut self, actor: u32, role: u32) {
        let db = self.scene.library.roles;
        if let Some(c) = self.character_mut(actor) {
            c.remove_role(db, role);
            c.roles_changed = true;
        }
    }
    fn add_relation(&mut self, actor: u32, other: u32, role: u32, value: u32) {
        if let Some(c) = self.character_mut(actor) {
            c.add_relation(other, role, value);
        }
    }
    fn remove_relation(&mut self, actor: u32, other: u32, role: u32, _value: u32) -> bool {
        self.character_mut(actor)
            .is_some_and(|c| c.remove_relation(other, role))
    }
    fn set_activity(&mut self, actor: u32, slot: i32, value: i32) {
        if let (Some(c), Ok(i)) = (self.character_mut(actor), usize::try_from(slot))
            && let Some(a) = c.activity.get_mut(i)
        {
            *a = value;
        }
    }
    fn trust(&self, actor: u32, other: u32) -> f32 {
        self.character(actor)
            .and_then(|c| c.contact(other))
            .map_or(0.0, |c| c.attitude)
    }
    fn concern(&self, actor: u32, other: u32) -> Option<f32> {
        self.character(actor)?.contact(other).map(|c| c.concern)
    }
    fn set_concern(&mut self, actor: u32, other: u32, concern: f32) {
        if let Some(c) = self
            .character_mut(actor)
            .and_then(|c| c.contacts.iter_mut().find(|k| k.other == other))
        {
            c.concern = concern;
        }
    }
    fn health_pool(&self, actor: u32) -> (f32, f32) {
        self.character(actor).map_or((0.0, 1.0), |c| c.health)
    }
    fn awake(&self, actor: u32) -> bool {
        self.character(actor).is_some_and(|c| c.awake)
    }
    fn has_item(&self, actor: u32, item: u32) -> bool {
        self.character(actor)
            .is_some_and(|c| c.items.contains(&item))
    }
    fn skill_techniques(&self, actor: u32) -> i32 {
        self.character(actor).map_or(0, |c| c.skill_techniques())
    }
    fn locale(&self) -> u32 {
        self.scene.locale
    }
    fn add_action(&mut self, actor: u32, action: u32) {
        if let Some(c) = self.character_mut(actor) {
            c.actions.push(action);
        }
    }
    fn show_inventory(&mut self, actor: u32) {
        self.scene.effects.push(Effect::ShowInventory(actor));
    }
    fn leave_dialogue(&mut self, actor: u32) {
        self.leaving.push(actor);
    }
    fn last_encounter(&self, slot: i32) -> i32 {
        let Some(conv) = self.conv.as_ref() else {
            return -1;
        };
        // A negative slot reads a conversation field not decoded; slot 0
        // is assumed.
        let i = slot.max(0) as usize;
        let ids = [conv.owner, conv.partner];
        let (Some(Some(Speaker::Character(me))), Some(Some(other))) =
            (conv.actors.get(i), ids.get(1 - i.min(1)))
        else {
            return -1;
        };
        self.character(*me)
            .and_then(|c| c.relation(*other))
            .map_or(-1, |r| r.last_encounter as i32)
    }
    fn activate_near(&mut self, actor: u32, role: i32, target: i32, mode: i32, topic: Name) {
        activate_near(
            self.scene,
            actor,
            role as u32,
            target as u32,
            mode as u32,
            topic,
        );
    }
    fn activate_with(&mut self, actor: u32, other: u32, topic: Name, role: u32, target: u32) {
        activate_with(self.scene, actor, other, topic, role, target, 1);
    }
    fn name(&self, actor: u32) -> String {
        self.character(actor)
            .map(|c| c.name.clone())
            .unwrap_or_default()
    }
    fn context_phrase(&self, actor: u32) -> String {
        self.character(actor)
            .map(|c| c.context_phrase.clone())
            .unwrap_or_default()
    }
    fn context_object(&self, actor: u32) -> String {
        self.character(actor)
            .and_then(|c| c.context_object.clone())
            .unwrap_or_else(|| "nothing".to_string())
    }
    fn game_event(&mut self, code: u32) {
        self.scene.effects.push(Effect::GameEvent(code));
    }
    fn checkpoint_save(&mut self) {
        self.scene.effects.push(Effect::CheckpointSave);
    }
    fn start_dialogue(&mut self, a: Option<u32>, b: Option<u32>, topic: Name) {
        self.scene.effects.push(Effect::StartDialogue(topic));
        self.start_requests.push((a, b, topic));
    }
    fn end_dialogue(&mut self) {
        *self.end_requested = true;
    }
    fn topic_used(&self, topic: Name) -> i32 {
        let (Some(graph), Some(conv)) = (self.graph(), self.conv.as_ref()) else {
            return 0;
        };
        graph
            .nodes
            .iter()
            .position(|n| n.as_ref().is_some_and(|n| n.title == topic))
            .and_then(|i| conv.words.get(i))
            .map_or(0, |&w| topics::used_time(w) as i32)
    }
    fn world_time(&self) -> u32 {
        self.scene.world_time
    }
    fn activate_topic(&mut self, topic: Name, mode: i32) -> bool {
        let library = self.scene.library;
        let Some(conv) = self.conv.as_mut() else {
            return false;
        };
        let Some(graph) = library.graph(conv.graph) else {
            return false;
        };
        let Some(i) = graph
            .nodes
            .iter()
            .position(|n| n.as_ref().is_some_and(|n| n.title == topic))
        else {
            return false;
        };
        let flags = graph.nodes[i].as_ref().map_or(0, |n| n.flags);
        let (Some(owner), Some(at)) = (conv.owner, conv.memory_key()) else {
            return false;
        };
        let Some(c) = self.scene.cast.get(owner) else {
            return false;
        };
        if let (Some(words), _) = topics::load(c, at, graph.nodes.len()) {
            conv.words = words;
        }
        let Some(w) = conv.words.get_mut(i) else {
            return false;
        };
        if !topics::activate(w, flags, mode as u32, self.scene.world_time) {
            return false;
        }
        let words = conv.words.clone();
        if let Some(c) = self.scene.cast.get_mut(owner) {
            topics::store(c, at, &words);
        }
        true
    }
    fn witness_role(&mut self, role: u32) {
        self.scene.effects.push(Effect::WitnessRole(role));
    }
}

/// `ActivateTopicN`: activate `topic` of `role`'s dialogue toward `target`
/// in the memory `actor` keeps of each contact that qualifies (every
/// contact holding a role R's dialogue has the topic, when `target` is 0).
pub fn activate_near(
    scene: &mut Scene<'_>,
    actor: u32,
    role: u32,
    target: u32,
    mode: u32,
    topic: Name,
) {
    let library = scene.library;
    let Some(me) = scene.cast.get(actor) else {
        return;
    };
    if !me.holds(role) {
        return;
    }
    let Some(rec) = library.roles.get(role) else {
        return;
    };
    let contacts: Vec<u32> = me.contacts.iter().map(|c| c.other).collect();
    let now = scene.world_time;
    // One word buffer serves every contact: where a contact has no block,
    // the previous contact's words are what gets updated and stored.
    let mut words: Option<Vec<u32>> = None;
    if target == 0 {
        for other in contacts {
            let Some(them) = scene.cast.get(other) else {
                continue;
            };
            let their_roles: Vec<u32> = them.roles.iter().map(|r| r.id).collect();
            for q in their_roles {
                let Some(graph) = rec.dialogue_for(q, 0) else {
                    continue;
                };
                let Some(n) = graph.find(&topic) else {
                    continue;
                };
                let at = MemoryKey {
                    other,
                    role: target,
                    key: role,
                };
                let flags = graph.node(n).map_or(0, |n| n.flags);
                let hit = Hit {
                    count: graph.nodes.len(),
                    index: n - 1,
                    flags,
                    mode,
                    now,
                };
                if activate_word(scene, actor, at, hit, &mut words) {
                    report_invalid(scene, "Near Activation", actor);
                }
            }
        }
        return;
    }
    let Some(graph) = rec.dialogue_for(target, 0) else {
        return;
    };
    let Some(n) = graph.find(&topic) else {
        return;
    };
    let flags = graph.node(n).map_or(0, |n| n.flags);
    let relation = library
        .roles
        .get(target)
        .is_some_and(|r| r.is_relation_role_byte());
    for other in contacts {
        // The actor must hold the aimed-at role (or, for a relation role,
        // hold it toward the contact); the contact's roles do not matter.
        let ok = if relation {
            scene
                .cast
                .get(actor)
                .is_some_and(|c| c.holds_toward(other, target))
        } else {
            scene.cast.get(actor).is_some_and(|c| c.holds(target))
        };
        if ok {
            let at = MemoryKey {
                other,
                role: target,
                key: role,
            };
            let hit = Hit {
                count: graph.nodes.len(),
                index: n - 1,
                flags,
                mode,
                now,
            };
            if activate_word(scene, actor, at, hit, &mut words) {
                report_invalid(scene, "Near Activation", actor);
            }
        }
    }
}

/// `Actor.ActivateTopic`: activate `topic` of relation role `role`'s
/// dialogue aimed at `target`, in `actor`'s memory of `other` (only a
/// relation role's dialogues are reached this way).
pub fn activate_with(
    scene: &mut Scene<'_>,
    actor: u32,
    other: u32,
    topic: Name,
    role: u32,
    target: u32,
    mode: u32,
) {
    let library = scene.library;
    let Some(rec) = library.roles.get(role) else {
        return;
    };
    if !rec.is_relation_role_byte() {
        return;
    }
    let Some(graph) = rec.dialogue_for(target, 0) else {
        return;
    };
    let Some(n) = graph.find(&topic) else {
        return;
    };
    let flags = graph.node(n).map_or(0, |n| n.flags);
    let at = MemoryKey {
        other,
        role,
        key: target,
    };
    let hit = Hit {
        count: graph.nodes.len(),
        index: n - 1,
        flags,
        mode,
        now: scene.world_time,
    };
    if activate_word(scene, actor, at, hit, &mut None) {
        report_invalid(scene, "Activate with", actor);
    }
}

/// A witnessed event (`GameEvent`'s memory flashes): for each contact of
/// A witnessed event (`GameEvent`'s memory flashes): for every contact of
/// `actor` that holds `target` (and, when `role` is a relation role, holds
/// it toward `actor`), activate `topic` of `role`'s dialogue aimed at
/// `target` in that contact's memory of `actor`. When `role` is not a
/// relation role, `actor` must hold it. The word is filed under entry
/// `role`, key `target`, exactly as the original stores it (for a
/// non-relation role that is not where the dialogue search looks, a quirk
/// of the original worth keeping).
pub fn witness_event(
    scene: &mut Scene<'_>,
    actor: u32,
    role: u32,
    target: u32,
    mode: u32,
    topic: Name,
) {
    let library = scene.library;
    let Some(rec) = library.roles.get(role) else {
        return;
    };
    let relation = rec.is_relation_role_byte();
    let Some(me) = scene.cast.get(actor) else {
        return;
    };
    if !relation && !me.holds(role) {
        return;
    }
    let Some(graph) = rec.dialogue_for(target, 0) else {
        return;
    };
    let Some(n) = graph.find(&topic) else {
        return;
    };
    let flags = graph.node(n).map_or(0, |n| n.flags);
    let contacts: Vec<u32> = me.contacts.iter().map(|c| c.other).collect();
    let mut words = None;
    for other in contacts {
        let witness = scene
            .cast
            .get(other)
            .is_some_and(|c| c.holds(target) && (!relation || c.holds_toward(actor, role)));
        if !witness {
            continue;
        }
        let at = MemoryKey {
            other: actor,
            role,
            key: target,
        };
        let hit = Hit {
            count: graph.nodes.len(),
            index: n - 1,
            flags,
            mode,
            now: scene.world_time,
        };
        if activate_word(scene, other, at, hit, &mut words) {
            report_invalid(scene, "Witness Event", actor);
        }
    }
}

/// A broadcast event (a hit, a fall, a spell): every contact of `actor`
/// that `sees` it activates `topic` in the first of its dialogues toward
/// `actor` that has it (the dialogue search's order, without the last
/// relation-role pass).
pub fn broadcast(
    scene: &mut Scene<'_>,
    actor: u32,
    topic: Name,
    mut sees: impl FnMut(u32, u32) -> bool,
) {
    let contacts: Vec<u32> = scene
        .cast
        .get(actor)
        .map(|c| c.contacts.iter().map(|k| k.other).collect())
        .unwrap_or_default();
    for witness in contacts {
        if sees(witness, actor) {
            witness_activate(scene, witness, actor, topic, 1);
        }
    }
}

/// One witness of a broadcast: walk its dialogues toward `actor` and stop
/// at the first whose `topic` changed.
pub fn witness_activate(
    scene: &mut Scene<'_>,
    witness: u32,
    actor: u32,
    topic: Name,
    mode: u32,
) -> bool {
    let library = scene.library;
    let (Some(a), Some(b)) = (scene.cast.get(witness), scene.cast.get(actor)) else {
        return false;
    };
    let a_roles: Vec<u32> = a.roles.iter().map(|r| r.id).collect();
    let b_roles: Vec<u32> = b.roles.iter().map(|r| r.id).collect();
    let rel: Vec<(u32, Vec<Memory>)> = a
        .relation(actor)
        .map(|r| {
            r.roles
                .iter()
                .map(|e| (e.role, e.memories.clone()))
                .collect()
        })
        .unwrap_or_default();
    let block_of = |entry_role: u32, key: u32| -> Option<Memory> {
        rel.iter()
            .find(|(r, _)| *r == entry_role)
            .and_then(|(_, m)| m.iter().find(|m| m.key == key).cloned())
    };
    let is_relation = |r: u32| library.roles.get(r).is_some_and(|r| r.is_relation_role());
    let try_one = |scene: &mut Scene<'_>, g: GraphRef, r: u32, block: Option<Memory>| -> bool {
        let Some(d) = library.graph(g) else {
            return false;
        };
        let Some(n) = d.find(&topic) else {
            return false;
        };
        let mut words = topics::from_block(block.as_ref(), d.nodes.len());
        let flags = d.node(n).map_or(0, |n| n.flags);
        if !topics::activate(&mut words[n - 1], flags, mode, scene.world_time) {
            return false;
        }
        if d.target != 0
            && let Some(c) = scene.cast.get_mut(witness)
        {
            let at = MemoryKey {
                other: actor,
                role: d.target,
                key: r,
            };
            topics::store(c, at, &words);
        }
        report_invalid(scene, "Broadcast Event", witness);
        true
    };
    for (i, &r) in a_roles.iter().enumerate() {
        for &q in &b_roles {
            if let Some(g) = library.aimed(r, q) {
                let target = library.graph(g).map_or(0, |d| d.target);
                if try_one(scene, g, r, block_of(target, r)) {
                    return true;
                }
            }
        }
        for (e, mems) in &rel {
            if !is_relation(*e) {
                continue;
            }
            if let Some(g) = library.aimed(r, *e) {
                let key = scene.cast.get(witness).map_or(0, |c| c.role_list_word(i));
                let block = mems.iter().find(|m| m.key == key).cloned();
                if try_one(scene, g, r, block) {
                    return true;
                }
            }
        }
        if let Some(g) = library.aimed(r, 0)
            && try_one(scene, g, r, block_of(0, r))
        {
            return true;
        }
    }
    false
}

/// One activation of a node word.
#[derive(Debug, Clone, Copy)]
struct Hit {
    count: usize,
    index: usize,
    flags: u32,
    mode: u32,
    now: u32,
}

/// Load the words into `words` (kept as they are when there is no block,
/// zeros when there were none yet), activate one, store on a change.
fn activate_word(
    scene: &mut Scene<'_>,
    owner: u32,
    at: MemoryKey,
    hit: Hit,
    words: &mut Option<Vec<u32>>,
) -> bool {
    let Some(c) = scene.cast.get_mut(owner) else {
        return false;
    };
    match topics::load(c, at, hit.count) {
        (Some(w), _) => *words = Some(w),
        (None, _) => {
            if words.is_none() {
                *words = Some(vec![0; hit.count]);
            }
        }
    }
    let Some(buf) = words.as_mut() else {
        return false;
    };
    if let Some(w) = buf.get_mut(hit.index)
        && topics::activate(w, hit.flags, hit.mode, hit.now)
    {
        topics::store(c, at, buf);
        return true;
    }
    false
}

/// Report the original's invalid-role-identifier message for `who` after an
/// event store (a guess: the original's validator is not in the packet, so
/// this checks every role against the database).
fn report_invalid(scene: &mut Scene<'_>, title: &str, who: u32) {
    if role_set_invalid(scene, who) {
        scene.effects.push(Effect::Report(
            title.to_string(),
            "Invalid role identifier.".to_string(),
        ));
    }
}

/// Whether `who` holds a role the database does not have. The original's
/// role-set validation reports a character whose roles cannot be resolved
/// (a guess: the routine's exact test is not in the packet).
fn role_set_invalid(scene: &Scene<'_>, who: u32) -> bool {
    scene.cast.get(who).is_none_or(|c| {
        c.roles
            .iter()
            .any(|r| scene.library.roles.get(r.id).is_none())
    })
}

impl Engine {
    fn conv(&self, id: usize) -> Option<&Conversation> {
        self.conversations.get(id).and_then(Option::as_ref)
    }

    fn conv_mut(&mut self, id: usize) -> Option<&mut Conversation> {
        self.conversations.get_mut(id).and_then(Option::as_mut)
    }

    /// The conversation `who` is in.
    pub fn current_of(&self, who: u32) -> Option<usize> {
        self.current
            .iter()
            .find(|(c, _)| *c == who)
            .map(|&(_, id)| id)
    }

    fn set_current(&mut self, who: u32, id: Option<usize>) {
        self.current.retain(|(c, _)| *c != who);
        if let Some(id) = id {
            self.current.push((who, id));
        }
    }

    fn release(&mut self, id: usize) {
        if let Some(c) = self.conv_mut(id) {
            c.refs -= 1;
            if c.refs < 1 {
                self.conversations[id] = None;
            }
        }
    }

    fn register(&mut self, id: usize) {
        self.active.push(id);
    }

    /// Next line index drawn from `n` (`seed × 0x8088405 + 1`, top bits).
    fn random(&mut self, n: usize) -> usize {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        ((u64::from(self.seed) * n as u64) >> 32) as usize
    }

    /// Run a script outside any conversation (a role's init or encounter
    /// script, an interaction condition, a used object's script) with
    /// these actor slots and running role; returns `result`.
    pub fn run_script(
        &mut self,
        scene: &mut Scene<'_>,
        actors: Vec<Option<u32>>,
        role: u32,
        text: &[u8],
    ) -> i32 {
        let mut ctx = Context::new(actors);
        ctx.role = role;
        self.last_run = None;
        let mut requests = Vec::new();
        let mut host = SceneHost {
            scene,
            conv: None,
            leaving: &mut self.leaving,
            end_requested: &mut self.end_requested,
            start_requests: &mut requests,
        };
        let r = interp::run(&mut host, &mut ctx, text);
        self.run_start_requests(scene, requests);
        self.settle(scene);
        r
    }

    /// Run a script of conversation `id`; returns `result`.
    fn run(&mut self, scene: &mut Scene<'_>, id: usize, text: &[u8]) -> i32 {
        if text.is_empty() {
            return 0;
        }
        self.last_run = Some(id);
        let Some(conv) = self.conversations.get_mut(id).and_then(Option::as_mut) else {
            return 0;
        };
        let mut ctx = std::mem::replace(&mut conv.ctx, Context::new(Vec::new()));
        let mut requests = Vec::new();
        let mut host = SceneHost {
            scene,
            conv: Some(conv),
            leaving: &mut self.leaving,
            end_requested: &mut self.end_requested,
            start_requests: &mut requests,
        };
        let r = interp::run(&mut host, &mut ctx, text);
        if let Some(conv) = self.conv_mut(id) {
            conv.ctx = ctx;
        }
        self.run_start_requests(scene, requests);
        r
    }

    /// `{…}` substitution with conversation `id`'s actors. As in the
    /// original, a substituted expression counts as the conversation whose
    /// script last ran (`EndDialogue` inside it closes that one).
    fn substitute(&mut self, scene: &mut Scene<'_>, id: usize, text: &[u8]) -> Vec<u8> {
        self.last_run = Some(id);
        let Some(conv) = self.conversations.get_mut(id).and_then(Option::as_mut) else {
            return text.to_vec();
        };
        let mut ctx = std::mem::replace(&mut conv.ctx, Context::new(Vec::new()));
        let mut requests = Vec::new();
        let mut host = SceneHost {
            scene,
            conv: Some(conv),
            leaving: &mut self.leaving,
            end_requested: &mut self.end_requested,
            start_requests: &mut requests,
        };
        let out = interp::substitute(&mut host, &mut ctx, text);
        if let Some(conv) = self.conv_mut(id) {
            conv.ctx = ctx;
        }
        self.run_start_requests(scene, requests);
        out
    }

    /// Start the dialogues `StartDialogue` asked for (the original starts
    /// them inside the call; the request runs when the script returns).
    fn run_start_requests(
        &mut self,
        scene: &mut Scene<'_>,
        requests: Vec<(Option<u32>, Option<u32>, Name)>,
    ) {
        for (a, b, topic) in requests {
            if let (Some(a), Some(b)) = (a, b) {
                self.start_dialogue(scene, a, b, topic);
            }
        }
    }

    /// Apply what scripts deferred: leaving characters end their
    /// conversations; `EndDialogue` closes the panel of the last run.
    fn settle(&mut self, scene: &mut Scene<'_>) {
        for who in std::mem::take(&mut self.leaving) {
            self.leave(scene, who);
        }
        if std::mem::take(&mut self.end_requested)
            && let Some(id) = self.last_run
            && self.panel.open
            && scene.player.and_then(|p| self.current_of(p)) == Some(id)
        {
            self.close_panel(scene);
        }
    }

    /// End every live conversation `who` is in.
    pub fn leave(&mut self, scene: &mut Scene<'_>, who: u32) {
        for i in 0..self.active.len() {
            let id = self.active[i];
            let Some(c) = self.conv(id) else { continue };
            if c.state != State::Ended
                && (c.actors.get(1) == Some(&Some(Speaker::Character(who)))
                    || c.actors.first() == Some(&Some(Speaker::Character(who))))
            {
                self.end(scene, id);
            }
        }
    }

    /// Find a dialogue for the request: A→B, then B→A.
    pub fn search(&mut self, scene: &mut Scene<'_>, mut req: Request) -> bool {
        req.swapped = false;
        let (a, b) = (req.a, req.b);
        if self.search_one(scene, &req, a, b) {
            return true;
        }
        req.swapped = true;
        self.search_one(scene, &req, b, a)
    }

    /// One direction of the search, owner `a`, partner `b`.
    pub fn search_one(&mut self, scene: &mut Scene<'_>, req: &Request, a: u32, b: u32) -> bool {
        let library = scene.library;
        let (Some(ca), Some(cb)) = (scene.cast.get(a), scene.cast.get(b)) else {
            return false;
        };
        let a_roles: Vec<u32> = ca.roles.iter().map(|r| r.id).collect();
        let b_roles: Vec<u32> = cb.roles.iter().map(|r| r.id).collect();
        let rel: Vec<(u32, Vec<Memory>)> = ca
            .relation(b)
            .map(|r| {
                r.roles
                    .iter()
                    .map(|e| (e.role, e.memories.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let block_of = |entry_role: u32, key: u32| -> Option<Memory> {
            rel.iter()
                .find(|(r, _)| *r == entry_role)
                .and_then(|(_, m)| m.iter().find(|m| m.key == key).cloned())
        };
        let is_relation = |r: u32| library.roles.get(r).is_some_and(|r| r.is_relation_role());
        for (i, &r) in a_roles.iter().enumerate() {
            for &q in &b_roles {
                let Some(g) = library.aimed(r, q) else {
                    continue;
                };
                let target = library.graph(g).map_or(0, |d| d.target);
                // The memory block is looked up under the word the stored
                // role list has at the role's slot (12 bytes per entry), as
                // the original reads it.
                let key = scene.cast.get(a).map_or(0, |c| c.role_list_word(i));
                let block = block_of(target, key);
                if self.open(scene, req, g, r, block.as_ref()) {
                    return true;
                }
            }
            for (e, mems) in &rel {
                if !is_relation(*e) {
                    continue;
                }
                let Some(g) = library.aimed(r, *e) else {
                    continue;
                };
                // The key is read from the role list as if every entry
                // were 12 bytes long.
                let key = scene.cast.get(a).map_or(0, |c| c.role_list_word(i));
                let block = mems.iter().find(|m| m.key == key).cloned();
                if self.open(scene, req, g, r, block.as_ref()) {
                    return true;
                }
            }
            if let Some(g) = library.aimed(r, 0)
                && self.open(scene, req, g, r, None)
            {
                return true;
            }
        }
        // Every dialogue of each relation role held toward B. The walk does
        // not move past a relation role, so the first one met is visited
        // for every remaining slot.
        let mut k = 0;
        for _ in 0..rel.len() {
            let (e, mems) = &rel[k];
            if !is_relation(*e) {
                k += 1;
                continue;
            }
            let Some(rec) = library.roles.get(*e) else {
                continue;
            };
            for (entry, x) in rec.entries.iter().enumerate() {
                if x.kind & exa_formats::roles::SECTION_DIALOGUE == 0 || x.dialogue.is_none() {
                    continue;
                }
                let block = mems.iter().find(|m| m.key == x.target).cloned();
                let g = GraphRef::Entry { role: *e, entry };
                if self.open(scene, req, g, *e, block.as_ref()) {
                    return true;
                }
            }
        }
        false
    }

    /// Open (or reuse) the conversation of `graph` for the request, then
    /// run its purpose.
    fn open(
        &mut self,
        scene: &mut Scene<'_>,
        req: &Request,
        graph: GraphRef,
        role: u32,
        block: Option<&Memory>,
    ) -> bool {
        let library = scene.library;
        let Some(d) = library.graph(graph) else {
            return false;
        };
        let (owner, partner) = if req.swapped {
            (req.b, req.a)
        } else {
            (req.a, req.b)
        };
        let actors = vec![
            Some(Speaker::Character(owner)),
            Some(Speaker::Character(partner)),
        ];
        let existing = self.active.iter().copied().find(|&id| {
            self.conv(id).is_some_and(|c| {
                c.state != State::Ended
                    && c.graph == graph
                    && c.actors.get(..2) == Some(&actors[..])
            })
        });
        let id = match existing {
            Some(id) => {
                if let Some(c) = self.conv_mut(id) {
                    c.refs += 1;
                }
                id
            }
            None => {
                self.conversations
                    .push(Some(Conversation::new(graph, actors)));
                self.conversations.len() - 1
            }
        };
        let arousal = scene.cast.get(req.a).map_or(0.0, |c| c.arousal);
        let relation = library
            .roles
            .get(role)
            .is_some_and(|r| r.is_relation_role());
        let Some(c) = self.conv_mut(id) else {
            return false;
        };
        c.mood_code = req.mood;
        c.arousal = arousal;
        c.owner = Some(owner);
        c.partner = Some(partner);
        (c.key, c.id) = if relation {
            (d.target, role)
        } else {
            (role, d.target)
        };
        c.role = role;
        c.words = topics::from_block(block, d.nodes.len());
        c.bind();
        self.purpose(scene, id, req)
    }

    fn purpose(&mut self, scene: &mut Scene<'_>, id: usize, req: &Request) -> bool {
        match &req.purpose {
            Purpose::Greet => self.start_or_release(scene, id, None, Filter::Greeting),
            Purpose::Start(name) => {
                let name = (name.0[0] != 0).then_some(*name);
                self.start_or_release(scene, id, name, Filter::Calm)
            }
            Purpose::Reply => self.start_or_release(scene, id, None, Filter::Calm),
            Purpose::Events => {
                let mut handled = self.events(scene, id);
                if self.start(scene, id, None, Filter::Calm) {
                    self.register(id);
                    handled = true;
                } else {
                    self.release(id);
                }
                handled
            }
            Purpose::EventsOnly => {
                self.events(scene, id);
                self.release(id);
                self.event_mask == 0
            }
            Purpose::Named(name) => {
                let library = scene.library;
                let graph = self.conv(id).and_then(|c| library.graph(c.graph));
                let mut handled = false;
                if let Some(graph) = graph {
                    for (i, n) in graph.nodes.iter().enumerate() {
                        if n.as_ref().is_some_and(|n| n.title == *name)
                            && self.event_node(scene, id, i + 1, self.event_subject)
                        {
                            handled = true;
                            break;
                        }
                    }
                }
                self.release(id);
                handled
            }
            Purpose::Topics => self.collect_topics(scene, id),
        }
    }

    fn start_or_release(
        &mut self,
        scene: &mut Scene<'_>,
        id: usize,
        name: Option<Name>,
        filter: Filter,
    ) -> bool {
        if self.start(scene, id, name, filter) {
            self.register(id);
            true
        } else {
            self.release(id);
            false
        }
    }

    /// Run the event nodes the pending mask names; each bit is cleared once
    /// a node of that name was tried (the mask belongs to the event, so a
    /// later dialogue of the same search sees it cleared).
    fn events(&mut self, scene: &mut Scene<'_>, id: usize) -> bool {
        let library = scene.library;
        let Some(graph) = self.conv(id).and_then(|c| library.graph(c.graph)) else {
            return false;
        };
        let mut handled = false;
        for (bit, title) in EVENTS {
            if self.event_mask & bit == 0 {
                continue;
            }
            let name = Name::from_bytes(title.as_bytes());
            for (i, n) in graph.nodes.iter().enumerate() {
                if n.as_ref().is_some_and(|n| n.title == name) {
                    if self.event_node(scene, id, i + 1, self.event_subject) {
                        handled = true;
                    }
                    self.event_mask &= !bit;
                }
            }
        }
        handled
    }

    /// Start at the named node, or at the first entry node the filter and
    /// the topic gate allow that has something to say.
    pub fn start(
        &mut self,
        scene: &mut Scene<'_>,
        id: usize,
        name: Option<Name>,
        filter: Filter,
    ) -> bool {
        self.offering = false;
        let library = scene.library;
        let Some(graph) = self.conv(id).and_then(|c| library.graph(c.graph)) else {
            return false;
        };
        match name {
            None => {
                for (i, n) in graph.nodes.iter().enumerate() {
                    let Some(n) = n else { continue };
                    if n.flags & (NODE_ENTRY | NODE_NOT_ENTRY) != NODE_ENTRY {
                        continue;
                    }
                    let Some(c) = self.conv(id) else { return false };
                    let pass = match filter {
                        Filter::Greeting => calm_any(n),
                        Filter::Calm => calm_enough(n, c.arousal),
                    };
                    let word = c.words.get(i).copied().unwrap_or(0);
                    if !pass || !topics::available(word, n.flags, scene.world_time) {
                        continue;
                    }
                    self.set_cursor(id, CursorSlot::Started, i + 1);
                    if self.say(scene, id, CursorSlot::Started, true) {
                        return true;
                    }
                }
                false
            }
            Some(name) => {
                let Some(i) = graph
                    .nodes
                    .iter()
                    .position(|n| n.as_ref().is_some_and(|n| n.title == name))
                else {
                    return false;
                };
                self.set_cursor(id, CursorSlot::Started, i + 1);
                self.say(scene, id, CursorSlot::Started, true)
            }
        }
    }

    fn set_cursor(&mut self, id: usize, slot: CursorSlot, node: usize) {
        if let Some(c) = self.conv_mut(id) {
            let cursor = Some(Cursor {
                node,
                path: Vec::new(),
                visited: false,
            });
            match slot {
                CursorSlot::Linked => c.linked = cursor,
                CursorSlot::Started => c.started = cursor,
            }
        }
    }

    /// Evaluate the group at `slot`: offer the player's lines, or speak the
    /// first branch that has a candidate line and take it.
    pub fn say(&mut self, scene: &mut Scene<'_>, id: usize, slot: CursorSlot, ai: bool) -> bool {
        let library = scene.library;
        let Some(conv) = self.conv(id) else {
            return false;
        };
        let Some(graph) = library.graph(conv.graph) else {
            return false;
        };
        let Some(cursor) = conv.cursor(slot).cloned() else {
            return false;
        };
        let actors = conv.actors.clone();
        let n = actors.len();
        if actors.first() != Some(&Some(Speaker::Narrator)) {
            let first = match actors.first() {
                Some(Some(Speaker::Character(a))) => Some(*a),
                _ => None,
            };
            let mut moods = [0u16; 4];
            if let (Some(a), Some(Some(Speaker::Character(b)))) = (first, actors.get(1)) {
                moods[1] = mood(scene.cast, a, *b);
            }
            for k in 1..n {
                if let (Some(Some(Speaker::Character(x))), Some(a)) = (actors.get(k), first) {
                    // Every further actor writes the same slot.
                    moods[2] = mood(scene.cast, *x, a);
                }
            }
            if let Some(c) = self.conv_mut(id) {
                c.moods = moods;
            }
        }
        let moods = self.conv(id).map_or([0; 4], |c| c.moods);
        let branches = group(graph, &cursor).to_vec();
        let mut produced = false;
        let mut buckets: Vec<Vec<Vec<u8>>> = vec![Vec::new(); n + 1];
        for (u, branch) in branches.iter().enumerate() {
            let mut passed = branch.condition.is_empty();
            let mut last_speaker = 0usize;
            for line in &branch.lines {
                let s = line.speaker as usize;
                last_speaker = s;
                if s == 0 || s > n {
                    continue;
                }
                let Some(who) = actors[s - 1] else { continue };
                let is_player = is_player(scene, who);
                if is_player {
                    if self.offering {
                        if !passed {
                            if self.run(scene, id, &branch.condition) < 1 {
                                break;
                            }
                            passed = true;
                        }
                        let text = self.substitute(scene, id, &line.text);
                        let topic = self.conv(id).map_or(0, |c| c.counter) as u32;
                        self.panel.entries.push(Choice {
                            text,
                            topic,
                            branch: u,
                        });
                        self.panel.can_pick = true;
                        if actors.first() == Some(&Some(Speaker::Narrator)) {
                            self.bubbles.hold_narrator();
                        }
                        produced = true;
                    }
                } else if ai {
                    let m = moods.get(s).copied().unwrap_or(0);
                    let f = branch.flags as u16;
                    let ok = f == 0 || (m & 0x1F0 != 0 && (m & 0x600) <= (f & 0x600));
                    if ok {
                        if !passed {
                            if self.run(scene, id, &branch.condition) < 1 {
                                break;
                            }
                            passed = true;
                        }
                        // Six lines per speaker fit.
                        if buckets[s].len() < 6 {
                            buckets[s].push(line.text.clone());
                        }
                    }
                }
            }
            if let Some(bucket) = buckets.iter().skip(1).find(|b| !b.is_empty()) {
                let pick = self.random(bucket.len());
                let text = bucket[pick].clone();
                let text = self.substitute(scene, id, &text);
                // The bubble goes to the speaker of the branch's last line.
                let s = last_speaker;
                let speaker = s
                    .checked_sub(1)
                    .and_then(|i| actors.get(i).copied().flatten());
                let listener = 2usize
                    .checked_sub(s)
                    .and_then(|i| actors.get(i).copied().flatten());
                if let Some(speaker) = speaker {
                    let due = self
                        .bubbles
                        .say(speaker, listener, &text, scene.now, scene.player);
                    if let Speaker::Character(c) = speaker {
                        self.set_current(c, Some(id));
                    }
                    let half = self.bubbles.last_duration() >> 1;
                    if let Some(c) = self.conv_mut(id) {
                        c.next_at = half.wrapping_add(due);
                    }
                }
                self.take(scene, id, slot, u);
                return true;
            }
        }
        self.settle(scene);
        produced
    }

    /// Take branch `k` of the group at `slot`: run its action, make its
    /// replies the group; an exhausted group records the node as used.
    pub fn take(&mut self, scene: &mut Scene<'_>, id: usize, slot: CursorSlot, k: usize) {
        let library = scene.library;
        let Some(conv) = self.conv_mut(id) else {
            return;
        };
        let switched = conv.current != Some(slot);
        if switched {
            conv.current = Some(slot);
            conv.jump = 0;
        }
        conv.state = State::Speaking;
        let Some(graph) = library.graph(conv.graph) else {
            return;
        };
        let Some(cursor) = conv.cursor_mut(slot) else {
            return;
        };
        if switched {
            cursor.visited = true;
        }
        let Some(branch) = group(graph, cursor).get(k).cloned() else {
            return;
        };
        cursor.path.push(k);
        let node = cursor.node;
        if branch.flags & 7 != 0 {
            conv.jump = branch.flags & 7;
        }
        if !branch.action.is_empty() {
            self.run(scene, id, &branch.action);
        }
        let narrator = self
            .conv(id)
            .is_some_and(|c| c.actors.first() == Some(&Some(Speaker::Narrator)));
        if branch.replies.is_empty() && !narrator {
            self.mark_used(scene, id, node);
        }
        self.settle(scene);
    }

    /// Record node `node` (from 1) as used now, if the memory entry exists.
    fn mark_used(&mut self, scene: &mut Scene<'_>, id: usize, node: usize) {
        let library = scene.library;
        let Some(conv) = self.conv_mut(id) else {
            return;
        };
        let (Some(owner), Some(at)) = (conv.owner, conv.memory_key()) else {
            return;
        };
        let count = library.graph(conv.graph).map_or(0, |g| g.nodes.len());
        let Some(c) = scene.cast.get(owner) else {
            return;
        };
        let (block, found) = topics::load(c, at, count);
        if !found {
            return;
        }
        let mut words = block.unwrap_or_else(|| conv.words.clone());
        if let Some(w) = node.checked_sub(1).and_then(|i| words.get_mut(i)) {
            *w = scene.world_time;
        }
        conv.words = words.clone();
        if let Some(c) = scene.cast.get_mut(owner) {
            topics::store(c, at, &words);
        }
    }

    /// One step of a speaking conversation.
    pub fn step(&mut self, scene: &mut Scene<'_>, id: usize) -> bool {
        let library = scene.library;
        let mine = scene.player.and_then(|p| self.current_of(p)) == Some(id);
        if mine {
            self.panel.entries.clear();
            self.panel.chosen = 0;
        }
        let Some(conv) = self.conv_mut(id) else {
            return false;
        };
        conv.state = State::Waiting;
        let out = conv.actors.iter().any(|a| match a {
            Some(Speaker::Character(c)) => scene.cast.get(*c).is_some_and(|c| c.out_of_action),
            _ => false,
        });
        if out {
            self.end(scene, id);
            return false;
        }
        let Some(conv) = self.conv(id) else {
            return false;
        };
        let Some(graph) = library.graph(conv.graph) else {
            return false;
        };
        let Some(slot) = conv.current else {
            return false;
        };
        let Some(cursor) = conv.cursor(slot).cloned() else {
            return false;
        };
        let mut result = false;
        if group(graph, &cursor).is_empty() {
            let mut moved = false;
            let jump = conv.jump as usize;
            if let Some(node) = graph.node(cursor.node)
                && jump > 0
                && jump <= node.links.len()
            {
                let link = (node.links[jump - 1] & 0xFFFF) as usize;
                if link != 0 && graph.node(link).is_some() {
                    self.set_cursor(id, CursorSlot::Linked, link);
                    if let Some(c) = self.conv_mut(id) {
                        c.counter = 0;
                    }
                    self.say(scene, id, CursorSlot::Linked, true);
                    if let Some(c) = self.conv_mut(id) {
                        c.current = Some(CursorSlot::Linked);
                    }
                    moved = true;
                    result = true;
                }
            }
            if moved {
                if let Some(c) = self.conv_mut(id) {
                    c.jump = 0;
                }
            } else {
                if self.panel.open && mine {
                    self.close_panel(scene);
                }
                self.end(scene, id);
            }
        } else {
            if let Some(c) = self.conv_mut(id) {
                c.counter = 0;
            }
            self.say(scene, id, slot, true);
            result = true;
        }
        result
    }

    /// End a conversation: the narrator's bubble closes in a second;
    /// otherwise the current node is recorded as used. (The node flag
    /// meant to prevent that is never consulted: the test reads the
    /// cursor's own word.)
    pub fn end(&mut self, scene: &mut Scene<'_>, id: usize) {
        let Some(conv) = self.conv(id) else { return };
        if conv.actors.first() == Some(&Some(Speaker::Narrator)) {
            self.bubbles.close_narrator(scene.now, 1000);
        } else if let Some(node) = conv.current_cursor().map(|c| c.node) {
            self.mark_used(scene, id, node);
        }
        if self.panel.open && scene.player.and_then(|p| self.current_of(p)) == Some(id) {
            self.close_panel(scene);
        }
        self.current.retain(|&(_, c)| c != id);
        if let Some(c) = self.conv_mut(id) {
            c.state = State::Ended;
        }
    }

    /// Run an event node with an optional third actor: the first branch
    /// whose condition passes runs its action and the node's use is
    /// recorded.
    pub fn event_node(
        &mut self,
        scene: &mut Scene<'_>,
        id: usize,
        node: usize,
        third: Option<u32>,
    ) -> bool {
        let library = scene.library;
        let Some(conv) = self.conv(id) else {
            return false;
        };
        let Some(graph) = library.graph(conv.graph) else {
            return false;
        };
        let Some(n) = graph.node(node) else {
            return false;
        };
        let word = conv.words.get(node - 1).copied().unwrap_or(0);
        if !topics::available(word, n.flags, scene.world_time) {
            return false;
        }
        let branches = n.branches.clone();
        // The third slot is added to the conversation but not to the
        // interpreter, which keeps the two actors it was bound with.
        if let Some(c) = self.conv_mut(id) {
            c.state = State::Speaking;
            c.actors.truncate(2);
            c.actors.push(third.map(Speaker::Character));
        }
        let mut handled = false;
        for b in &branches {
            if !b.condition.is_empty() && self.run(scene, id, &b.condition) < 1 {
                continue;
            }
            if !b.action.is_empty() {
                self.run(scene, id, &b.action);
            }
            let has_id = self.conv(id).is_some_and(|c| c.id != 0);
            if has_id {
                self.mark_used(scene, id, node);
            }
            handled = true;
            break;
        }
        if let Some(c) = self.conv_mut(id) {
            c.actors.truncate(2);
            c.state = State::Waiting;
        }
        // The original validates both actors' role sets after the event and
        // names the actor slot that is invalid.
        let actors: Vec<Option<Speaker>> =
            self.conv(id).map_or_else(Vec::new, |c| c.actors.clone());
        for (slot, actor) in actors.iter().take(2).enumerate() {
            if let Some(Speaker::Character(who)) = actor
                && role_set_invalid(scene, *who)
            {
                scene.effects.push(Effect::Report(
                    "Process Event".to_string(),
                    format!("Actor {slot} invalid role identifier."),
                ));
            }
        }
        self.settle(scene);
        handled
    }

    /// Whether node `node` is offered as a topic: non-empty, open by its
    /// memory, not the target of any link, and with a player line whose
    /// condition passes.
    fn offer_topic(&mut self, scene: &mut Scene<'_>, id: usize, node: usize) -> bool {
        let library = scene.library;
        let Some(conv) = self.conv(id) else {
            return false;
        };
        let Some(graph) = library.graph(conv.graph) else {
            return false;
        };
        let Some(n) = graph.node(node) else {
            return false;
        };
        let word = conv.words.get(node - 1).copied().unwrap_or(0);
        if !topics::available(word, n.flags, scene.world_time) {
            return false;
        }
        let linked = graph
            .nodes
            .iter()
            .flatten()
            .any(|m| m.links.iter().any(|&l| (l & 0xFFFF) as usize == node));
        if linked {
            return false;
        }
        if conv.current.is_some() && node == conv.picked {
            return false;
        }
        if let Some(c) = self.conv_mut(id) {
            c.counter += 1;
        }
        // Evaluated with a scratch cursor.
        let saved = self.conv(id).and_then(|c| c.linked.clone());
        self.set_cursor(id, CursorSlot::Linked, node);
        let ok = self.say(scene, id, CursorSlot::Linked, false);
        if let Some(c) = self.conv_mut(id) {
            c.linked = saved;
            if !ok {
                c.counter -= 1;
            }
        }
        ok
    }

    /// Collect the topics of a found dialogue into the panel.
    fn collect_topics(&mut self, scene: &mut Scene<'_>, id: usize) -> bool {
        let library = scene.library;
        let Some(graph) = self.conv(id).and_then(|c| library.graph(c.graph)) else {
            return false;
        };
        let before = self.panel.topics.len();
        if let Some(c) = self.conv_mut(id) {
            c.counter = before as i32;
        }
        for (i, n) in graph.nodes.iter().enumerate() {
            if self.offer_topic(scene, id, i + 1)
                && let Some(n) = n
            {
                self.panel.topics.push((id, n.title));
            }
        }
        if self.panel.topics.len() > before {
            self.panel.conversations.push(id);
        } else {
            self.release(id);
        }
        false
    }

    /// The replies of a conversation the partner holds with the player.
    fn replies(&mut self, scene: &mut Scene<'_>, partner: Speaker) -> bool {
        self.panel.conversation = None;
        let player = scene.player.map(Speaker::Character);
        for i in 0..self.active.len() {
            let id = self.active[i];
            let Some(c) = self.conv(id) else { continue };
            if c.state == State::Ended
                || c.actors.first() != Some(&Some(partner))
                || c.actors.get(1).copied().flatten() != player
                || c.current.is_none()
            {
                continue;
            }
            let Some(slot) = c.current else { continue };
            self.panel.conversation = Some(id);
            if let Some(c) = self.conv_mut(id) {
                c.counter = 0;
            }
            if self.say(scene, id, slot, false) {
                if let Some(c) = self.conv_mut(id) {
                    c.refs += 1;
                }
                return true;
            }
            self.panel.conversation = None;
        }
        false
    }

    /// Rebuild the panel's list for `partner`: replies first, then topics,
    /// with `[Other]` switching between them when both exist.
    pub fn rebuild(&mut self, scene: &mut Scene<'_>, partner: Speaker) -> bool {
        self.panel.entries.clear();
        self.panel.chosen = 0;
        self.panel.conversation = None;
        if let Some(p) = scene.player {
            self.set_current(p, None);
        }
        self.offering = true;
        let mut replies = 0;
        let mut result = false;
        if self.replies(scene, partner) {
            replies = self.panel.entries.len();
            if !self.panel.other {
                if let Some(p) = scene.player {
                    self.set_current(p, self.panel.conversation);
                }
                result = true;
            } else {
                self.panel.entries.clear();
                self.panel.chosen = 0;
            }
        }
        self.offering = false;
        let Speaker::Character(partner_id) = partner else {
            return !self.panel.entries.is_empty();
        };
        let Some(player) = scene.player else {
            return result;
        };
        if scene.cast.get(player).is_none_or(|c| c.roles.is_empty()) {
            return result;
        }
        self.panel.topics.clear();
        for id in std::mem::take(&mut self.panel.conversations) {
            self.release(id);
        }
        self.offering = true;
        self.search(
            scene,
            Request {
                a: player,
                b: partner_id,
                mood: 0,
                swapped: false,
                purpose: Purpose::Topics,
            },
        );
        self.offering = false;
        if replies > 0 {
            if !self.panel.other {
                self.panel.entries.truncate(replies);
                if !self.panel.topics.is_empty() {
                    self.push_other();
                }
                self.panel.topics.clear();
                if let Some(p) = scene.player {
                    self.set_current(p, self.panel.conversation);
                }
                return true;
            }
            if self.panel.topics.is_empty() {
                self.panel.other = false;
                self.panel.entries.truncate(replies);
                if let Some(p) = scene.player {
                    self.set_current(p, self.panel.conversation);
                }
            } else {
                self.panel.conversation = None;
                self.push_other();
            }
        }
        !self.panel.entries.is_empty()
    }

    fn push_other(&mut self) {
        self.panel.entries.push(Choice {
            text: b"[Other]".to_vec(),
            topic: 0xFFFF,
            branch: 0,
        });
        self.panel.can_pick = true;
    }

    /// The player talks to `partner`: open the panel with its list.
    pub fn talk(&mut self, scene: &mut Scene<'_>, partner: u32) {
        self.panel.open = true;
        self.panel.other = false;
        self.panel.chosen = 0;
        if self.rebuild(scene, Speaker::Character(partner)) {
            self.panel.partner = Some(Speaker::Character(partner));
        } else {
            self.panel.open = false;
        }
    }

    /// The player picks entry `n`.
    pub fn pick(&mut self, scene: &mut Scene<'_>, n: usize) {
        if !self.panel.can_pick || n >= self.panel.entries.len() {
            return;
        }
        self.panel.can_pick = false;
        let choice = self.panel.entries[n].clone();
        let Some(partner) = self.panel.partner else {
            return;
        };
        if choice.topic == 0xFFFF {
            self.panel.other = !self.panel.other;
            self.rebuild(scene, partner);
            return;
        }
        let mut due = scene.now;
        if partner == Speaker::Narrator {
            self.bubbles.close_narrator(scene.now, 1000);
        } else if let Some(p) = scene.player {
            due = self.bubbles.say(
                Speaker::Character(p),
                Some(partner),
                &choice.text,
                scene.now,
                scene.player,
            );
        }
        self.panel.chosen = n + 1;
        let conv = if choice.topic < 1 {
            let Some(id) = self.panel.conversation else {
                return;
            };
            if let Some(slot) = self.conv(id).and_then(|c| c.current) {
                self.take(scene, id, slot, choice.branch);
            }
            Some(id)
        } else {
            self.pick_topic(scene, choice.topic as usize - 1, choice.branch)
        };
        let Some(id) = conv else { return };
        if let Some(p) = scene.player {
            self.set_current(p, Some(id));
        }
        let last = self.bubbles.last_duration();
        if let Some(c) = self.conv_mut(id) {
            c.next_at = last.wrapping_add(due);
        }
        if self.is_final(scene, id) {
            self.close_panel(scene);
        }
    }

    /// Start topic `k` of the list at branch `branch`.
    fn pick_topic(&mut self, scene: &mut Scene<'_>, k: usize, branch: usize) -> Option<usize> {
        let library = scene.library;
        let &(id, title) = self.panel.topics.get(k)?;
        let graph = self.conv(id).and_then(|c| library.graph(c.graph))?;
        let i = graph
            .nodes
            .iter()
            .position(|n| n.as_ref().is_some_and(|n| n.title == title))?;
        self.set_cursor(id, CursorSlot::Linked, i + 1);
        self.panel.conversation = Some(id);
        self.take(scene, id, CursorSlot::Linked, branch);
        if let Some(c) = self.conv_mut(id) {
            c.picked = i + 1;
            c.refs += 1;
        }
        self.register(id);
        Some(id)
    }

    /// Whether the conversation has nothing after this (no replies left,
    /// no link to follow).
    pub fn is_final(&self, scene: &Scene<'_>, id: usize) -> bool {
        let Some(c) = self.conv(id) else { return true };
        let Some(graph) = scene.library.graph(c.graph) else {
            return true;
        };
        let Some(cursor) = c.current_cursor() else {
            return true;
        };
        if group(graph, cursor).iter().any(|b| !b.replies.is_empty()) {
            return false;
        }
        let Some(node) = graph.node(cursor.node) else {
            return true;
        };
        !(!node.links.is_empty() && (node.links.len() > 1 || node.links[0] != 0))
    }

    pub fn close_panel(&mut self, scene: &mut Scene<'_>) {
        if let Some(p) = scene.player {
            self.set_current(p, None);
        }
        self.panel.open = false;
        self.panel.entries.clear();
    }

    /// Move the panel's highlight (wheel or keys): back while above the
    /// first entry, forward while below the last (from none, to the first).
    pub fn move_highlight(&mut self, forward: bool) {
        if !self.panel.open {
            return;
        }
        let h = self.panel.highlight;
        if !forward {
            if h > 1 {
                self.panel.highlight = h - 1;
            }
        } else if h < self.panel.entries.len() {
            self.panel.highlight = (h + 1).max(1);
        }
    }

    /// Pick the highlighted entry.
    pub fn confirm(&mut self, scene: &mut Scene<'_>) {
        let h = self.panel.highlight;
        if h > 0 && h <= self.panel.entries.len() {
            self.pick(scene, h - 1);
        }
    }

    /// End every conversation (a session ends or a save loads): live ones
    /// end with their topic recorded, all are released, a pending
    /// narration is dropped and the panel closes.
    pub fn end_all(&mut self, scene: &mut Scene<'_>) {
        for i in 0..self.active.len() {
            let id = self.active[i];
            if self.conv(id).is_some_and(|c| c.state != State::Ended) {
                self.end(scene, id);
            }
            self.release(id);
        }
        self.active.clear();
        self.narration = None;
        self.close_panel(scene);
    }

    /// Start the narrator at `topic` (after `delay` ms when non-zero).
    pub fn narrate(&mut self, scene: &mut Scene<'_>, topic: Name, mut delay: u32) {
        let (Some(player), Some(_)) = (scene.player, scene.library.narrator) else {
            return;
        };
        for i in 0..self.active.len() {
            let id = self.active[i];
            if self
                .conv(id)
                .is_some_and(|c| c.actors.first() == Some(&Some(Speaker::Narrator)))
            {
                self.end(scene, id);
            }
        }
        if self
            .bubbles
            .list
            .iter()
            .any(|b| b.speaker == Speaker::Narrator)
        {
            self.bubbles.close_all_narrator(scene.now, 500);
            delay = delay.max(500);
        }
        if delay != 0 {
            self.narration = Some((scene.now.wrapping_add(delay), Narration::Topic(topic)));
            return;
        }
        let count = scene.library.narrator.map_or(0, |g| g.nodes.len());
        let mut conv = Conversation::new(
            GraphRef::Narrator,
            vec![Some(Speaker::Narrator), Some(Speaker::Character(player))],
        );
        conv.partner = Some(player);
        conv.words = vec![0; count];
        conv.bind();
        self.conversations.push(Some(conv));
        let id = self.conversations.len() - 1;
        if self.start(scene, id, Some(topic), Filter::Calm) {
            self.register(id);
            self.panel.open = true;
            self.panel.other = false;
            self.panel.partner = Some(Speaker::Narrator);
            if !self.rebuild(scene, Speaker::Narrator) {
                self.panel.open = false;
            }
            self.narrator_done = false;
        } else {
            self.conversations[id] = None;
        }
    }

    /// Show narrator text without a graph (after `delay` ms when non-zero).
    pub fn narrate_text(&mut self, scene: &mut Scene<'_>, text: &[u8], mut delay: u32) {
        if self
            .bubbles
            .list
            .iter()
            .any(|b| b.speaker == Speaker::Narrator)
        {
            self.bubbles.close_all_narrator(scene.now, 500);
            delay = delay.max(500);
        }
        if delay == 0 {
            let listener = scene.player.map(Speaker::Character);
            self.bubbles
                .say(Speaker::Narrator, listener, text, scene.now, scene.player);
        } else {
            self.narration = Some((
                scene.now.wrapping_add(delay),
                Narration::Text(text.to_vec()),
            ));
        }
    }

    /// Whether `who` is in a live conversation.
    pub fn engaged(&self, who: u32, any_state: bool) -> bool {
        self.active.iter().any(|&id| {
            self.conv(id).is_some_and(|c| {
                (any_state || c.state != State::Ended)
                    && c.actors.contains(&Some(Speaker::Character(who)))
            })
        })
    }

    /// An AI greets its contact `b` (when calm and free).
    pub fn greet(&mut self, scene: &mut Scene<'_>, a: u32, b: u32) -> bool {
        if scene.cast.get(a).is_none_or(|c| c.arousal > GREET_AROUSAL) || self.engaged(a, false) {
            return false;
        }
        let mood = mood_code(scene.cast, a, b);
        self.search(
            scene,
            Request {
                a,
                b,
                mood,
                swapped: false,
                purpose: Purpose::Greet,
            },
        )
    }

    /// `StartDialogue(topic)` for the pair `a`, `b`.
    pub fn start_dialogue(&mut self, scene: &mut Scene<'_>, a: u32, b: u32, topic: Name) -> bool {
        if scene.cast.get(a).is_none_or(|c| c.arousal > START_AROUSAL) || self.engaged(a, true) {
            return false;
        }
        self.search(
            scene,
            Request {
                a,
                b,
                mood: 0,
                swapped: false,
                purpose: Purpose::Start(topic),
            },
        )
    }

    /// Run the node titled `name` as an event between `a` and `b` (looting
    /// a body runs one this way).
    pub fn named_event(
        &mut self,
        scene: &mut Scene<'_>,
        a: u32,
        b: u32,
        name: Name,
        subject: Option<u32>,
    ) -> bool {
        self.event_subject = subject;
        let req = Request {
            a,
            b,
            mood: 0,
            swapped: false,
            purpose: Purpose::Named(name),
        };
        self.search_one(scene, &req, a, b)
    }

    /// AI events of `a` about `b` (bits of [`EVENTS`]; `subject` is the
    /// event's character). With `a` already talking only event nodes run;
    /// otherwise the events, then a reply; failing both A→B, a reply B→A.
    /// Returns what is left of the mask.
    pub fn ai_events(
        &mut self,
        scene: &mut Scene<'_>,
        a: u32,
        b: u32,
        mask: u32,
        subject: Option<u32>,
    ) -> u32 {
        self.event_mask = mask;
        self.event_subject = subject;
        if self.engaged(a, false) {
            let req = Request {
                a,
                b,
                mood: 0,
                swapped: false,
                purpose: Purpose::EventsOnly,
            };
            self.search_one(scene, &req, a, b);
            return self.event_mask;
        }
        let mood = mood_code(scene.cast, a, b);
        let req = Request {
            a,
            b,
            mood,
            swapped: false,
            purpose: Purpose::Events,
        };
        if !self.search_one(scene, &req, a, b) {
            let req = Request {
                swapped: true,
                purpose: Purpose::Reply,
                ..req
            };
            self.search_one(scene, &req, b, a);
        }
        self.event_mask
    }

    /// Per frame: a due narrator message fires; due conversations step;
    /// idle ones end after 30 s; ended ones are released.
    pub fn update(&mut self, scene: &mut Scene<'_>) {
        if let Some((due, _)) = self.narration
            && due < scene.now
            && let Some((_, n)) = self.narration.take()
        {
            match n {
                Narration::Topic(t) => self.narrate(scene, t, 0),
                Narration::Text(t) => self.narrate_text(scene, &t, 0),
            }
        }
        for i in 0..self.active.len() {
            let id = self.active[i];
            let Some(c) = self.conv(id) else { continue };
            if self.narrator_done && c.actors.first() == Some(&Some(Speaker::Narrator)) {
                self.end(scene, id);
                continue;
            }
            match c.state {
                State::Speaking if c.next_at < scene.now => {
                    let mine = scene.player.and_then(|p| self.current_of(p)) == Some(id);
                    self.offering = self.panel.open && mine;
                    self.step(scene, id);
                    self.offering = false;
                }
                State::Waiting
                    if u64::from(c.next_at) + u64::from(REPLY_WAIT) < u64::from(scene.now) =>
                {
                    self.end(scene, id);
                }
                _ => {}
            }
        }
        let active = std::mem::take(&mut self.active);
        for id in active {
            if self.conv(id).is_some_and(|c| c.state == State::Ended) {
                self.release(id);
            } else if self.conv(id).is_some() {
                self.active.push(id);
            }
        }
        self.bubbles.update(scene.now);
    }
}

/// Which entry nodes a start may choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Calm class 0 (AI greetings).
    Greeting,
    /// Calm class from 1, the stored arousal within its threshold.
    Calm,
}
