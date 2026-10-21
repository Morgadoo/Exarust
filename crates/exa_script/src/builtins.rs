//! The script classes (`Actor`, `Role`), the global functions, and the
//! [`Host`] the built-ins act through.
//!
//! Names are looked up by exact length and bytes, in registration order.
//! `Actor` methods: `Role`, `LeaveDialogue`, `InZone`, `ActivateTopicN`,
//! `ShowInventory`, `ChangeRole`, `Health`, `LastEnc`, `SkillTecs`,
//! `InLocale`, `HasRole`, `AddAction`, `AddRole`, `RemoveRole`,
//! `AddRelation`, `RemRelation`, `ActivateTopic`, `Trust`, `Concern`,
//! `AddConcern`, `Awake`, `HasItem`; properties `Activity` and
//! `IdleActivity` (written through to the AI; read back they give 0, the
//! class's table being never filled); text members `Name`,
//! `ContextPhrase`, `ContextObj`. `Role` exposes the running role's
//! variables. Globals: `GameEvent`, `CheckpointSave`, `StartDialogue`,
//! `ChangeRole`, `EndDialogue`, `TopicUsed`, `WorldTime`,
//! `ActivateTopic`, `WitnessRole`.
//!
//! Most methods act on `Actor[n]` with a negative slot taken as 0; `Role`,
//! `InZone`, `ActivateTopicN`, `AddRelation`, `RemRelation`,
//! `ActivateTopic`, `Trust`, `Concern`, `AddConcern` and `Awake` use the
//! slot as it is. A slot with no actor does nothing and gives 0 (the
//! original reads an empty slot).
//!
//! Evidence: the `Actor` class registration and each method, the `Role`
//! class (variable load, write, commit), the unit initialisers that
//! register the globals, and the constants they use (REA on `Exanima.exe`
//! 0.9.5).

use exa_formats::roles::Name;

use crate::interp::{self, Context, RoleObject};

/// What scripts act on. Every hook has a neutral default so a host only
/// implements what it has.
pub trait Host {
    /// A character as the host knows it.
    type Actor: Copy + PartialEq + std::fmt::Debug;

    /// A script error ("Script Error", "Invalid class identifier.").
    fn report(&mut self, _title: &str, _message: &str) {}

    /// Whether `actor` holds `role` (inherited parents included).
    fn holds_role(&self, _actor: Self::Actor, _role: u32) -> bool {
        false
    }
    /// The declared variable names of `role` (empty for an unknown id).
    fn role_variables(&self, _role: u32) -> Vec<Name> {
        Vec::new()
    }
    /// The stored values of `role` on `actor`; `None` if not held.
    fn role_values(&self, _actor: Self::Actor, _role: u32) -> Option<Vec<i32>> {
        None
    }
    /// Store `values` as `actor`'s variables of `role` if it holds it (the
    /// entry takes this many variables), and have the AI apply roles anew.
    fn set_role_values(&mut self, _actor: Self::Actor, _role: u32, _values: &[i32]) {}
    /// Add a role (and the parents it inherits); the AI applies roles anew.
    fn add_role(&mut self, _actor: Self::Actor, _role: u32) {}
    fn remove_role(&mut self, _actor: Self::Actor, _role: u32) {}
    /// Give `actor` the relation role `role` toward `other`.
    fn add_relation(&mut self, _actor: Self::Actor, _other: Self::Actor, _role: u32, _value: u32) {}
    /// Whether a relation role was removed.
    fn remove_relation(
        &mut self,
        _actor: Self::Actor,
        _other: Self::Actor,
        _role: u32,
        _value: u32,
    ) -> bool {
        false
    }
    /// `slot` 0 = Activity, 1 = IdleActivity (other slots do nothing).
    fn set_activity(&mut self, _actor: Self::Actor, _slot: i32, _value: i32) {}
    /// The effective attitude of `actor` toward `other`.
    fn trust(&self, _actor: Self::Actor, _other: Self::Actor) -> f32 {
        0.0
    }
    /// `actor`'s concern for its contact `other`; `None` with no contact.
    fn concern(&self, _actor: Self::Actor, _other: Self::Actor) -> Option<f32> {
        None
    }
    fn set_concern(&mut self, _actor: Self::Actor, _other: Self::Actor, _concern: f32) {}
    /// The health pool scripts see and its maximum.
    fn health_pool(&self, _actor: Self::Actor) -> (f32, f32) {
        (0.0, 1.0)
    }
    /// Not knocked down (motion state `0xA00`) and not asleep (state flag
    /// `0x10`).
    fn awake(&self, _actor: Self::Actor) -> bool {
        true
    }
    fn has_item(&self, _actor: Self::Actor, _item: u32) -> bool {
        false
    }
    /// Techniques learnt: the sum over the six skills of points / 5000.
    fn skill_techniques(&self, _actor: Self::Actor) -> i32 {
        0
    }
    /// The current locale id (`locales.rdb`).
    fn locale(&self) -> u32 {
        0
    }
    /// Whether `actor` stands inside the zone marker `zone`.
    fn in_zone(&self, _actor: Self::Actor, _zone: i32) -> bool {
        false
    }
    fn add_action(&mut self, _actor: Self::Actor, _action: u32) {}
    fn show_inventory(&mut self, _actor: Self::Actor) {}
    /// End every dialogue `actor` is in, not recording the current topic.
    fn leave_dialogue(&mut self, _actor: Self::Actor) {}
    /// When `actor` last encountered the partner of the running dialogue
    /// (`slot` is the term's actor slot); −1 without one.
    fn last_encounter(&self, _slot: i32) -> i32 {
        -1
    }
    /// Activate topic `topic` of `role`'s dialogue toward `target` in
    /// `actor`'s contacts (`ActivateTopicN`).
    fn activate_near(
        &mut self,
        _actor: Self::Actor,
        _role: i32,
        _target: i32,
        _mode: i32,
        _topic: Name,
    ) {
    }
    /// Activate topic `topic` in the relation of `actor` with `other`
    /// (`Actor.ActivateTopic`).
    fn activate_with(
        &mut self,
        _actor: Self::Actor,
        _other: Self::Actor,
        _topic: Name,
        _role: u32,
        _target: u32,
    ) {
    }
    fn name(&self, _actor: Self::Actor) -> String {
        String::new()
    }
    /// The phrase the AI last set (empty when none).
    fn context_phrase(&self, _actor: Self::Actor) -> String {
        String::new()
    }
    /// The name of the item `actor` is busy with, "nothing" without one.
    fn context_object(&self, _actor: Self::Actor) -> String {
        "nothing".to_string()
    }
    fn game_event(&mut self, _code: u32) {}
    fn checkpoint_save(&mut self) {}
    /// `StartDialogue(topic)` for the script's first two actor slots (the
    /// original passes the request to the role unit, which finds a graph
    /// between them).
    fn start_dialogue(
        &mut self,
        _actor0: Option<Self::Actor>,
        _actor1: Option<Self::Actor>,
        _topic: Name,
    ) {
    }
    fn end_dialogue(&mut self) {}
    /// The stored use time of `topic` in the running dialogue, 0 when it is
    /// activated or counting.
    fn topic_used(&self, _topic: Name) -> i32 {
        0
    }
    /// World time in seconds.
    fn world_time(&self) -> u32 {
        0
    }
    /// Activate `topic` of the running dialogue; whether it took.
    fn activate_topic(&mut self, _topic: Name, _mode: i32) -> bool {
        false
    }
    /// Give every character near the script's object `role`.
    fn witness_role(&mut self, _role: u32) {}
}

/// A script class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Actor,
    Role,
}

pub fn class(name: &[u8]) -> Option<Class> {
    match name {
        b"Actor" => Some(Class::Actor),
        b"Role" => Some(Class::Role),
        _ => None,
    }
}

/// `Actor` methods, in registration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Role,
    LeaveDialogue,
    InZone,
    ActivateTopicN,
    ShowInventory,
    ChangeRole,
    Health,
    LastEnc,
    SkillTecs,
    InLocale,
    HasRole,
    AddAction,
    AddRole,
    RemoveRole,
    AddRelation,
    RemRelation,
    ActivateTopic,
    Trust,
    Concern,
    AddConcern,
    Awake,
    HasItem,
}

pub const METHODS: [(&str, Method); 22] = [
    ("Role", Method::Role),
    ("LeaveDialogue", Method::LeaveDialogue),
    ("InZone", Method::InZone),
    ("ActivateTopicN", Method::ActivateTopicN),
    ("ShowInventory", Method::ShowInventory),
    ("ChangeRole", Method::ChangeRole),
    ("Health", Method::Health),
    ("LastEnc", Method::LastEnc),
    ("SkillTecs", Method::SkillTecs),
    ("InLocale", Method::InLocale),
    ("HasRole", Method::HasRole),
    ("AddAction", Method::AddAction),
    ("AddRole", Method::AddRole),
    ("RemoveRole", Method::RemoveRole),
    ("AddRelation", Method::AddRelation),
    ("RemRelation", Method::RemRelation),
    ("ActivateTopic", Method::ActivateTopic),
    ("Trust", Method::Trust),
    ("Concern", Method::Concern),
    ("AddConcern", Method::AddConcern),
    ("Awake", Method::Awake),
    ("HasItem", Method::HasItem),
];

pub fn actor_method(name: &[u8]) -> Option<Method> {
    METHODS
        .iter()
        .find(|(n, _)| n.as_bytes() == name)
        .map(|&(_, m)| m)
}

/// `Actor` properties: their slot.
pub fn actor_property(name: &[u8]) -> Option<i32> {
    match name {
        b"Activity" => Some(0),
        b"IdleActivity" => Some(1),
        _ => None,
    }
}

/// `Actor` members read as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Name,
    ContextPhrase,
    ContextObj,
}

pub fn actor_text(name: &[u8]) -> Option<Text> {
    match name {
        b"Name" => Some(Text::Name),
        b"ContextPhrase" => Some(Text::ContextPhrase),
        b"ContextObj" => Some(Text::ContextObj),
        _ => None,
    }
}

/// Global functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Global {
    GameEvent,
    CheckpointSave,
    StartDialogue,
    ChangeRole,
    EndDialogue,
    TopicUsed,
    WorldTime,
    ActivateTopic,
    WitnessRole,
}

pub fn global(name: &[u8]) -> Option<Global> {
    Some(match name {
        b"GameEvent" => Global::GameEvent,
        b"CheckpointSave" => Global::CheckpointSave,
        b"StartDialogue" => Global::StartDialogue,
        b"ChangeRole" => Global::ChangeRole,
        b"EndDialogue" => Global::EndDialogue,
        b"TopicUsed" => Global::TopicUsed,
        b"WorldTime" => Global::WorldTime,
        b"ActivateTopic" => Global::ActivateTopic,
        b"WitnessRole" => Global::WitnessRole,
        _ => return None,
    })
}

/// The `Role` object of `role` on `owner`: the role's variables, filled
/// from the stored values as far as both go.
pub fn role_object<H: Host>(host: &H, owner: H::Actor, role: u32, slot: usize) -> RoleObject {
    let names = host.role_variables(role);
    let mut values = vec![0; names.len()];
    if let Some(stored) = host.role_values(owner, role) {
        for (v, s) in values.iter_mut().zip(stored) {
            *v = s;
        }
    }
    RoleObject {
        role,
        slot,
        names,
        values,
        dirty: false,
    }
}

/// `Health`: the pool as a percentage of its maximum, rounded to nearest
/// (ties to even), 0 when the pool is empty.
pub fn health_percent(pool: f32, max: f32) -> i32 {
    if pool > 0.0 {
        (pool / max * 100.0).round_ties_even() as i32
    } else {
        0
    }
}

/// Concern is kept within 0..=15.
pub const CONCERN_MAX: f32 = 15.0;

/// The original's clamp: above 15 becomes 15, anything not at least 0
/// (an unordered value included) becomes 0.
pub fn clamp_concern(c: f32) -> f32 {
    if c > CONCERN_MAX {
        CONCERN_MAX
    } else if c >= 0.0 {
        c
    } else {
        0.0
    }
}

/// Reach of the `WitnessRole` grant, cm (18 m: the original compares the
/// squared distance against 3.24e6).
pub const WITNESS_RADIUS_CM: f32 = 1800.0;

/// Whether a character at `(dx, dy, dz)` from the actor is close enough for
/// `WitnessRole`; line of sight is still the caller's to test.
pub fn witness_range(dx: f32, dy: f32, dz: f32) -> bool {
    dx * dx + dy * dy + dz * dz <= 3.24e6
}

/// The characters a `WitnessRole` grant reaches: those within
/// [`WITNESS_RADIUS_CM`] (the original's squared-distance test) for which
/// `sees` answers, in the given order.
pub fn witnesses<'a>(
    actor: [f32; 3],
    candidates: impl IntoIterator<Item = (u32, [f32; 3])> + 'a,
    mut sees: impl FnMut(u32) -> bool,
) -> Vec<u32> {
    candidates
        .into_iter()
        .filter(|(id, [x, y, z])| {
            witness_range(x - actor[0], y - actor[1], z - actor[2]) && sees(*id)
        })
        .map(|(id, _)| id)
        .collect()
}

/// Type word of a zone marker object, as `InZone` looks it up in the object
/// database.
pub const ZONE_MARKER_TYPE: u32 = 0x8000_AC07;
/// The vertical slack of the zone-marker test.
pub const ZONE_HEIGHT: f32 = 0.1;

/// One zone marker as `InZone` tests it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneMarker {
    /// The object's database type.
    pub kind: u32,
    /// The marker's zone id.
    pub id: u32,
    /// Horizontal distance from the character, squared.
    pub horizontal_sq: f32,
    /// The marker's horizontal radius.
    pub radius: f32,
    /// Vertical distance from the character.
    pub vertical: f32,
}

/// The original's `InZone(wanted_locale, zone)`: the current locale must be
/// the wanted one, and some marker of type [`ZONE_MARKER_TYPE`] with the
/// wanted id must have the character within its horizontal radius and no
/// more than [`ZONE_HEIGHT`] above or below it.
pub fn zone_match(locale: u32, wanted_locale: i32, zone: i32, markers: &[ZoneMarker]) -> bool {
    locale as i32 == wanted_locale && zone_marker_matches(zone, markers)
}

/// The marker part of `InZone` (the locale is checked before this): some
/// marker of type [`ZONE_MARKER_TYPE`] with `zone`'s id has the character
/// within its radius and height.
pub fn zone_marker_matches(zone: i32, markers: &[ZoneMarker]) -> bool {
    markers.iter().any(|m| {
        m.kind == ZONE_MARKER_TYPE
            && m.id as i32 == zone
            && m.horizontal_sq <= m.radius * m.radius
            && m.vertical <= ZONE_HEIGHT
    })
}

/// The original's `Round` helper, a floor (the instruction's note on the
/// pseudo-code's ROUND): `Trust` and `Concern` round down.
fn round(v: f32) -> i32 {
    v.floor() as i32
}

pub(crate) fn call_actor<H: Host>(
    host: &mut H,
    ctx: &mut Context<H::Actor>,
    text: &[u8],
    method: Method,
) -> i32 {
    use Method::*;
    // The slot as written (for the methods that do not clamp it).
    let raw = ctx.actor_at(ctx.actor);
    match method {
        Role => {
            let Some(actor) = raw else { return 0 };
            let role = interp::arg_id(ctx, text, 0);
            if host.holds_role(actor, role) {
                let obj = role_object(host, actor, role, ctx.actor.max(0) as usize);
                ctx.status = 1;
                ctx.chained = Some(obj);
            } else {
                ctx.status = 0xFFFF;
            }
            0
        }
        LeaveDialogue => {
            if let Some(a) = ctx.subject() {
                host.leave_dialogue(a);
            }
            0
        }
        InZone => {
            if host.locale() as i32 != interp::arg_int(ctx, text, 0) {
                return 0;
            }
            let zone = interp::arg_int(ctx, text, 1);
            raw.is_some_and(|a| host.in_zone(a, zone)) as i32
        }
        ActivateTopicN => {
            let topic = interp::arg_name(ctx, text, 2);
            let mode = interp::arg_int(ctx, text, 3);
            let target = interp::arg_int(ctx, text, 1);
            let role = interp::arg_int(ctx, text, 0);
            if let Some(a) = raw {
                host.activate_near(a, role, target, mode, topic);
            }
            1
        }
        ShowInventory => {
            if let Some(a) = ctx.subject() {
                host.show_inventory(a);
            }
            1
        }
        ChangeRole => {
            host.report(
                "Script Error",
                "Function \"Actor.ChangeRole\" does not exist.",
            );
            1
        }
        Health => ctx.subject().map_or(0, |a| {
            let (pool, max) = host.health_pool(a);
            health_percent(pool, max)
        }),
        LastEnc => host.last_encounter(ctx.actor),
        SkillTecs => ctx.subject().map_or(0, |a| host.skill_techniques(a)),
        InLocale => (host.locale() as i32 == interp::arg_int(ctx, text, 0)) as i32,
        HasRole => {
            let role = interp::arg_id(ctx, text, 0);
            ctx.subject().is_some_and(|a| host.holds_role(a, role)) as i32
        }
        AddAction => {
            let id = interp::arg_id(ctx, text, 0);
            if let Some(a) = ctx.subject() {
                host.add_action(a, id);
            }
            id as i32
        }
        AddRole | RemoveRole => {
            let role = interp::arg_id(ctx, text, 0);
            if let Some(a) = ctx.subject() {
                if method == AddRole {
                    host.add_role(a, role);
                } else {
                    host.remove_role(a, role);
                }
            }
            1
        }
        AddRelation | RemRelation => {
            let (Some(actor), Some(other)) = (raw, interp::arg_actor(ctx, text, 0)) else {
                return 0;
            };
            let role = interp::arg_id(ctx, text, 1);
            let value = if ctx.args.len() < 3 {
                0
            } else {
                interp::arg_id(ctx, text, 2)
            };
            if method == AddRelation {
                host.add_relation(actor, other, role, value);
                0
            } else {
                host.remove_relation(actor, other, role, value) as i32
            }
        }
        ActivateTopic => {
            let (Some(actor), Some(other)) = (raw, interp::arg_actor(ctx, text, 0)) else {
                return 0;
            };
            let topic = interp::arg_name(ctx, text, 1);
            let (role, target) = if ctx.args.len() < 3 {
                (0, 0)
            } else {
                let role = interp::arg_id(ctx, text, 2);
                let target = if ctx.args.len() < 4 {
                    0
                } else {
                    interp::arg_id(ctx, text, 3)
                };
                (role, target)
            };
            host.activate_with(actor, other, topic, role, target);
            0
        }
        Trust => {
            let (Some(actor), Some(other)) = (raw, interp::arg_actor(ctx, text, 0)) else {
                return 0;
            };
            if actor == other {
                100
            } else {
                round(host.trust(actor, other))
            }
        }
        Concern => {
            let (Some(actor), Some(other)) = (raw, interp::arg_actor(ctx, text, 0)) else {
                return 0;
            };
            host.concern(actor, other).map_or(0, round)
        }
        AddConcern => {
            let (Some(actor), Some(other)) = (raw, interp::arg_actor(ctx, text, 0)) else {
                return 0;
            };
            if let Some(c) = host.concern(actor, other) {
                let c = interp::arg_int(ctx, text, 1) as f32 + c;
                host.set_concern(actor, other, clamp_concern(c));
            }
            0
        }
        Awake => raw.is_some_and(|a| host.awake(a)) as i32,
        HasItem => {
            let id = interp::arg_id(ctx, text, 0);
            ctx.subject().is_some_and(|a| host.has_item(a, id)) as i32
        }
    }
}

pub(crate) fn call_actor_text<H: Host>(
    host: &mut H,
    ctx: &mut Context<H::Actor>,
    member: Text,
) -> Option<String> {
    let a = ctx.subject()?;
    Some(match member {
        Text::Name => host.name(a),
        Text::ContextPhrase => host.context_phrase(a),
        Text::ContextObj => host.context_object(a),
    })
}

pub(crate) fn call_global<H: Host>(
    host: &mut H,
    ctx: &mut Context<H::Actor>,
    text: &[u8],
    global: Global,
) -> i32 {
    match global {
        Global::GameEvent => {
            // The shipped scripts write the code as bare hex
            // (`GameEvent(00C10EC0);`), so the argument is read as
            // hexadecimal rather than the `$`-or-decimal id form.
            let code = ctx.args.first().map_or(0, |&(s, l)| {
                interp::hexadecimal(interp::arg_bytes(text, s, l))
            });
            host.game_event(code);
            0
        }
        Global::CheckpointSave => {
            host.checkpoint_save();
            0
        }
        Global::StartDialogue => {
            let a = ctx.actor_at(0);
            let b = ctx.actor_at(1);
            host.start_dialogue(a, b, interp::arg_name(ctx, text, 0));
            0
        }
        Global::ChangeRole => {
            host.report("Script Error", "Function \"ChangeRole\" does not exist.");
            0
        }
        Global::EndDialogue => {
            host.end_dialogue();
            1
        }
        Global::TopicUsed => host.topic_used(interp::arg_name(ctx, text, 0)),
        Global::WorldTime => host.world_time() as i32,
        Global::ActivateTopic => {
            let topic = interp::arg_name(ctx, text, 0);
            let mode = interp::arg_int(ctx, text, 1);
            host.activate_topic(topic, mode) as i32
        }
        Global::WitnessRole => {
            if ctx
                .args
                .first()
                .is_some_and(|&(s, l)| interp::arg_bytes(text, s, l) == b"Operative")
            {
                host.witness_role(interp::arg_id(ctx, text, 1));
            }
            0
        }
    }
}
