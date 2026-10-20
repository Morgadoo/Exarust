//! The role and dialogue script interpreter.
//!
//! The original walks the script text in one pass with a cursor; there is no
//! tokeniser and no syntax tree. Statements end with `;`. A term is a name
//! up to a space, `;`, `.`, `[`, `(` or `)`, optionally followed by `[n]`
//! (the actor slot, one digit) and `.member` (a class), `(args)` (a global
//! function) or nothing (`result`, a local variable or a decimal literal).
//! `? cond : stmt; / stmt;` and `{ … }` blocks are a conditional with an
//! optional else. Conditions compare (`=`, `!` for not-equal, `<`, `>`, or a
//! bare value > 0), chain left to right with `&` and `|`, and compare actor
//! references by identity. Expressions are one term, optionally negated
//! with `!` (low bit flipped) and one `+` or `-` term.
//!
//! The walk is reproduced character for character, quirks included: the
//! character after a closing `)` is consumed; a branch skipped by a false
//! condition continues one character past the `;` that ends it; `()` with
//! nothing inside is not seen as closed; the variable/property slot used by
//! an assignment is the one most recently looked up, even on the right-hand
//! side. Where the original reads past the text, the walk reads NUL. One
//! place where the original never advances (a second space before `=`) is
//! stepped over instead of hanging.
//!
//! Evidence: the statement loop, term, member call, argument, expression,
//! condition, conditional and block-skip routines of the script unit, the
//! class and global registries and the argument helpers (REA on
//! `Exanima.exe` 0.9.5).

use exa_formats::roles::Name;

use crate::builtins::{self, Class, Host};

/// What a term turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Term {
    /// Nothing usable (an unknown name, a non-number).
    None,
    /// A global function was called.
    Function,
    /// A value was read (literal, variable, method result, property).
    Value,
    /// `result`: assignable.
    Result,
    /// A local variable: assignable.
    Local,
    /// A class property: assignable.
    Property,
}

impl Term {
    fn assignable(self) -> bool {
        matches!(self, Term::Result | Term::Local | Term::Property)
    }
}

/// A local variable (a role's variables, declared before the run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Local {
    pub name: Name,
    pub value: i32,
}

/// The `Role` object a script reads and writes: the variables of one role
/// held by one actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleObject {
    pub role: u32,
    /// The actor slot whose role it is.
    pub slot: usize,
    pub names: Vec<Name>,
    pub values: Vec<i32>,
    /// Written since it was read.
    pub dirty: bool,
}

/// Where a pending assignment goes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    /// `Actor.Activity` / `Actor.IdleActivity`.
    ActorProperty,
    /// The run's own `Role.` object (committed when the run ends).
    RunRole,
    /// A role reached through `Actor[n].Role(id)`, committed at once.
    TemporaryRole(RoleObject),
}

/// The object a member is looked up on.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Object {
    Actor,
    RunRole,
    TemporaryRole(RoleObject),
}

/// Interpreter state that outlives one statement: the actors, the running
/// role, the locals and the cached `Role.` object.
#[derive(Debug, Clone)]
pub struct Context<A> {
    /// `Actor[0]`, `Actor[1]`, … (`None`: an empty slot).
    pub actors: Vec<Option<A>>,
    /// The role whose script runs (`Role.` refers to its variables on
    /// `Actor[0]`).
    pub role: u32,
    pub locals: Vec<Local>,
    run_role: Option<RoleObject>,
    // The cursor state of the run.
    pub(crate) pos: i32,
    pub(crate) value: i32,
    /// The `[n]` of the last term (a byte minus `'0'`, so possibly out of
    /// range).
    pub(crate) actor: i32,
    pub(crate) args: Vec<(i32, i32)>,
    /// What a method asked for: 1 = continue on the returned object,
    /// 0xFFFF = skip the rest of the term.
    pub(crate) status: u32,
    pub(crate) chained: Option<RoleObject>,
    reading: bool,
    pending: Option<Pending>,
    /// The variable or property slot last looked up.
    slot: i32,
}

impl<A: Copy + PartialEq> Context<A> {
    pub fn new(actors: Vec<Option<A>>) -> Self {
        Self {
            actors,
            role: 0,
            locals: Vec::new(),
            run_role: None,
            pos: 0,
            value: 0,
            actor: 0,
            args: Vec::new(),
            status: 0,
            chained: None,
            reading: false,
            pending: None,
            slot: 0,
        }
    }

    /// The actor in slot `i`, if the slot exists and is filled.
    pub fn actor_at(&self, i: i32) -> Option<A> {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.actors.get(i).copied().flatten())
    }

    /// The actor the current method acts on: the term's `[n]`, negative
    /// clamped to 0 as the built-ins do.
    pub(crate) fn subject(&mut self) -> Option<A> {
        if self.actor < 0 {
            self.actor = 0;
        }
        self.actor_at(self.actor)
    }
}

/// Run `text` as a statement list; returns `result` (0 unless assigned).
/// The run's `Role.` object is committed to `Actor[0]` if it was written.
pub fn run<H: Host>(host: &mut H, ctx: &mut Context<H::Actor>, text: &[u8]) -> i32 {
    let result = {
        let mut w = Walk { host, ctx, text };
        w.exec()
    };
    if let Some(role) = ctx.run_role.take()
        && role.dirty
        && let Some(owner) = ctx.actor_at(role.slot as i32)
    {
        host.set_role_values(owner, role.role, &role.values);
    }
    result
}

/// Evaluate one `Class[n].Member` text expression (the `{…}` groups of
/// dialogue lines) to text; `None` when nothing answers.
pub fn text_of<H: Host>(host: &mut H, ctx: &mut Context<H::Actor>, text: &[u8]) -> Option<String> {
    ctx.args.clear();
    ctx.pos = 0;
    let mut w = Walk { host, ctx, text };
    w.object_expression()
}

/// Replace every `{…}` group of a line with what its expression gives
/// (nothing when no member answers), the way spoken lines are prepared.
pub fn substitute<H: Host>(host: &mut H, ctx: &mut Context<H::Actor>, line: &[u8]) -> Vec<u8> {
    let mut s = line.to_vec();
    let mut i = 1usize; // 1-based, as the original walks its string
    while i <= s.len() {
        if s[i - 1] == b'{' {
            let mut j = i;
            loop {
                j += 1;
                if s.get(j - 1) == Some(&b'}') || j >= s.len() {
                    break;
                }
            }
            let inner = s[i..(j - 1).max(i)].to_vec();
            let replacement = text_of(host, ctx, &inner).unwrap_or_default().into_bytes();
            let end = j.min(s.len());
            s.splice(i - 1..end, replacement.iter().copied());
            i = i + replacement.len() - 1;
        }
        i += 1;
    }
    s
}

struct Walk<'a, H: Host> {
    host: &'a mut H,
    ctx: &'a mut Context<H::Actor>,
    text: &'a [u8],
}

impl<H: Host> Walk<'_, H> {
    fn len(&self) -> i32 {
        self.text.len().min(i32::MAX as usize) as i32
    }

    /// The byte at `i`, NUL outside the text.
    fn at(&self, i: i32) -> u8 {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.text.get(i).copied())
            .unwrap_or(0)
    }

    fn slice(&self, start: i32, len: i32) -> &[u8] {
        let start = start.clamp(0, self.len()) as usize;
        let end = (start as i64 + i64::from(len.max(0))).min(self.text.len() as i64) as usize;
        &self.text[start..end]
    }

    fn pos(&self) -> i32 {
        self.ctx.pos
    }

    fn set_pos(&mut self, p: i32) {
        self.ctx.pos = p;
    }

    /// Spaces and CRs, and a LF straight after either.
    fn skip_blanks(&mut self) {
        while matches!(self.at(self.pos()), b' ' | b'\r') {
            self.set_pos(self.pos() + 1);
            if self.at(self.pos()) == b'\n' {
                self.set_pos(self.pos() + 1);
            }
        }
    }

    /// Past spaces, consume `ch` if it is next.
    fn expect(&mut self, ch: u8) -> bool {
        let last = self.len() - 1;
        let mut i = self.pos();
        while i <= last {
            let c = self.at(i);
            if c == ch {
                self.set_pos(i + 1);
                return true;
            }
            if c != b' ' {
                break;
            }
            i += 1;
        }
        false
    }

    /// Arguments from the `(` at `*p`: each is the text from its first
    /// non-space up to its `,` or `)`. `*p` ends one past the `)`.
    fn parse_args(&mut self, p: &mut i32) {
        self.ctx.args.clear();
        *p += 1;
        let last = self.len() - 1;
        let mut i = *p;
        while i < last {
            let j = i + 1;
            let c = self.at(j);
            if c == b',' || c == b')' {
                while self.at(*p) == b' ' {
                    *p += 1;
                }
                self.ctx.args.push((*p, j - *p));
                *p = j + 1;
                if c == b')' {
                    return;
                }
            }
            i = j;
        }
    }

    /// One statement list (the whole script).
    fn exec(&mut self) -> i32 {
        let mut result = 0;
        self.ctx.value = -1;
        self.set_pos(0);
        loop {
            if self.pos() >= self.len() - 1 {
                return result;
            }
            self.skip_blanks();
            match self.at(self.pos()) {
                b'?' => {
                    self.set_pos(self.pos() + 1);
                    self.conditional();
                    continue;
                }
                b'/' => {
                    self.skip_block();
                    if self.pos() >= self.len() - 1 {
                        return result;
                    }
                }
                b'}' => {
                    self.set_pos(self.pos() + 1);
                    if self.at(self.pos()) == b' ' {
                        self.set_pos(self.pos() + 1);
                    }
                    if self.at(self.pos()) == b'/' {
                        self.skip_block();
                        if self.pos() >= self.len() - 1 {
                            return result;
                        }
                    }
                }
                _ => {}
            }
            self.ctx.actor = 0;
            self.ctx.reading = false;
            self.ctx.pending = None;
            let term = self.term();
            if !term.assignable() {
                continue;
            }
            while self.pos() < self.len() - 1 {
                match self.at(self.pos()) {
                    b'=' => {
                        self.ctx.reading = true;
                        self.set_pos(self.pos() + 1);
                        if self.expect(b'(') {
                            self.ctx.value = i32::from(self.condition());
                        } else {
                            self.term();
                        }
                        let value = self.ctx.value;
                        match term {
                            Term::Result => result = value,
                            Term::Local => {
                                if let Some(l) = usize::try_from(self.ctx.slot)
                                    .ok()
                                    .and_then(|i| self.ctx.locals.get_mut(i))
                                {
                                    l.value = value;
                                }
                            }
                            Term::Property => self.write_pending(value),
                            _ => {}
                        }
                        self.ctx.pending = None;
                        break;
                    }
                    b';' => {
                        self.set_pos(self.pos() + 1);
                        break;
                    }
                    b'/' => {
                        self.skip_block();
                        break;
                    }
                    // A space here never advances in the original (it
                    // hangs); step over it.
                    _ => self.set_pos(self.pos() + 1),
                }
            }
        }
    }

    fn write_pending(&mut self, value: i32) {
        let slot = self.ctx.slot;
        match self.ctx.pending.take() {
            Some(Pending::ActorProperty) => {
                if let Some(actor) = self.ctx.actor_at(self.ctx.actor) {
                    self.host.set_activity(actor, slot, value);
                }
            }
            Some(Pending::RunRole) => {
                if let Some(role) = self.ctx.run_role.as_mut()
                    && let Some(v) = usize::try_from(slot)
                        .ok()
                        .and_then(|i| role.values.get_mut(i))
                {
                    *v = value;
                    role.dirty = true;
                }
            }
            Some(Pending::TemporaryRole(mut role)) => {
                if let Some(v) = usize::try_from(slot)
                    .ok()
                    .and_then(|i| role.values.get_mut(i))
                {
                    *v = value;
                }
                if let Some(owner) = self.ctx.actor_at(role.slot as i32) {
                    self.host.set_role_values(owner, role.role, &role.values);
                }
            }
            None => {}
        }
    }

    /// Skip a statement (past its `;`). A `{ … }` met on the way is skipped
    /// to its first `}` (and onto a `;` right after it); the walk then goes
    /// on to the next `;`.
    fn skip_block(&mut self) {
        loop {
            if self.pos() >= self.len() {
                break;
            }
            match self.at(self.pos()) {
                b'{' => {
                    while self.pos() < self.len() {
                        if self.at(self.pos()) == b'}' {
                            if self.at(self.pos() + 1) == b';' {
                                self.set_pos(self.pos() + 1);
                            }
                            break;
                        }
                        self.set_pos(self.pos() + 1);
                    }
                }
                b';' => break,
                _ => self.set_pos(self.pos() + 1),
            }
        }
        self.set_pos(self.pos() + 1);
    }

    /// `? cond : then / else`.
    fn conditional(&mut self) {
        if self.condition() {
            while self.pos() < self.len() {
                if !matches!(self.at(self.pos()), b' ' | b':' | b'{') {
                    return;
                }
                self.set_pos(self.pos() + 1);
            }
            return;
        }
        // Skip the then-part: one statement, or a brace block.
        let last = self.len() - 1;
        let mut i = self.pos();
        let mut brace = false;
        while i <= last {
            match self.at(i) {
                b';' => {
                    self.set_pos(i + 1);
                    break;
                }
                b'{' => {
                    brace = true;
                    break;
                }
                _ => i += 1,
            }
        }
        if brace {
            while self.pos() < self.len() {
                if self.at(self.pos()) == b'}' {
                    self.set_pos(self.pos() + 1);
                    break;
                }
                self.set_pos(self.pos() + 1);
            }
        }
        // An else-part starts at a `/`; anything else is stepped past.
        let mut i = self.pos();
        while i <= last {
            let c = self.at(i);
            if c == b'/' {
                self.set_pos(i + 1);
                break;
            }
            if c != b' ' {
                self.set_pos(i + 1);
                return;
            }
            i += 1;
        }
        let mut i = self.pos();
        while i <= last {
            if self.at(i) == b'{' {
                self.set_pos(i + 1);
                return;
            }
            i += 1;
        }
    }

    /// A chain of comparisons joined by `&` / `|`, ended by `;` or `:`.
    fn condition(&mut self) -> bool {
        let mut acc = false;
        let mut op = 0;
        loop {
            if self.pos() >= self.len() - 1 {
                return acc;
            }
            let mut b = false;
            let last = self.len() - 1;
            if let Some(lhs) = self.actor_ref() {
                let mut i = self.pos();
                while i <= last {
                    match self.at(i) {
                        b'=' => {
                            self.set_pos(i + 1);
                            b = self.actor_ref() == Some(lhs);
                            break;
                        }
                        b'!' => {
                            self.set_pos(i + 1);
                            b = self.actor_ref() != Some(lhs);
                            break;
                        }
                        b' ' => i += 1,
                        _ => break,
                    }
                }
            } else {
                let a = self.sum();
                let mut i = self.pos();
                while i <= last {
                    let c = self.at(i);
                    let cmp: Option<fn(i32, i32) -> bool> = match c {
                        b'=' => Some(|a, v| a == v),
                        b'!' => Some(|a, v| a != v),
                        b'>' => Some(|a, v| a > v),
                        b'<' => Some(|a, v| a < v),
                        _ => None,
                    };
                    if let Some(cmp) = cmp {
                        self.set_pos(i + 1);
                        let v = self.sum();
                        self.ctx.value = v;
                        b = cmp(a, v);
                        break;
                    }
                    if c != b' ' {
                        b = a > 0;
                        break;
                    }
                    i += 1;
                }
            }
            acc = match op {
                1 => b & acc,
                2 => b | acc,
                _ => b,
            };
            while self.pos() < self.len() {
                match self.at(self.pos()) {
                    b' ' | b')' => self.set_pos(self.pos() + 1),
                    b';' | b':' => {
                        self.set_pos(self.pos() + 1);
                        return acc;
                    }
                    _ => break,
                }
            }
            if self.pos() > last {
                continue;
            }
            let mut i = self.pos();
            while i <= last {
                match self.at(i) {
                    b'&' => {
                        self.set_pos(i + 1);
                        self.expect(b'(');
                        op = 1;
                        break;
                    }
                    b'|' => {
                        self.set_pos(i + 1);
                        self.expect(b'(');
                        op = 2;
                        break;
                    }
                    b' ' => i += 1,
                    _ => break,
                }
            }
        }
    }

    /// `Actor` or `Actor[n]` standing alone (not followed by a member):
    /// the actor; otherwise nothing, the cursor untouched.
    fn actor_ref(&mut self) -> Option<H::Actor> {
        self.skip_blanks();
        let p = self.pos();
        if self.slice(p, 5) != b"Actor" {
            return None;
        }
        let q = p + 5;
        let slot = if self.at(q) == b'[' {
            if self.at(q + 3) == b'.' {
                return None;
            }
            let n = i32::from(self.at(q + 1)) - i32::from(b'0');
            self.set_pos(p + 8);
            n
        } else {
            let c = self.at(q);
            if c == b'.' || c > b':' {
                return None;
            }
            self.set_pos(q);
            0
        };
        self.ctx.actor_at(slot)
    }

    /// A term, optionally negated with `!` and with one `+` or `-` term.
    fn sum(&mut self) -> i32 {
        let negate = self.at(self.pos()) == b'!';
        if negate {
            self.set_pos(self.pos() + 1);
        }
        self.ctx.reading = true;
        self.term();
        if negate {
            self.ctx.value = !self.ctx.value & 1;
        }
        let a = self.ctx.value;
        let last = self.len() - 1;
        let mut i = self.pos();
        while i <= last {
            match self.at(i) {
                b'-' => {
                    self.set_pos(i + 1);
                    self.term();
                    return a.wrapping_sub(self.ctx.value);
                }
                b'+' => {
                    self.set_pos(i + 1);
                    self.term();
                    return a.wrapping_add(self.ctx.value);
                }
                b' ' => i += 1,
                _ => return a,
            }
        }
        a
    }

    /// One term; its value lands in `ctx.value`.
    fn term(&mut self) -> Term {
        self.skip_blanks();
        let start = self.pos();
        let mut end = start;
        while end < self.len() {
            if matches!(self.at(end), b' ' | b';' | b'.' | b'[' | b'(' | b')') {
                break;
            }
            end += 1;
        }
        let nlen = end - start;
        if self.at(end) == b'[' {
            self.ctx.actor = i32::from(self.at(end + 1)) - i32::from(b'0');
            end += 3;
        }
        let name = self.slice(start, nlen).to_vec();
        if self.at(end) == b'.' {
            let class = builtins::class(&name);
            if class.is_none() {
                self.host
                    .report("Script Error", "Invalid class identifier.");
            }
            self.set_pos(end + 1);
            return match class {
                Some(Class::Actor) => self.member(Object::Actor),
                Some(Class::Role) => {
                    if self.run_role() {
                        self.member(Object::RunRole)
                    } else {
                        self.host
                            .report("Script Error", "Invalid class identifier.");
                        Term::None
                    }
                }
                None => Term::None,
            };
        }
        if let Some(t) = self.variable(&name) {
            self.set_pos(end + 1);
            if self.ctx.reading {
                self.ctx.value = usize::try_from(self.ctx.slot)
                    .ok()
                    .and_then(|i| self.ctx.locals.get(i))
                    .map_or(0, |l| l.value);
                return Term::Value;
            }
            return t;
        }
        self.ctx.args.clear();
        if self.at(end) == b'(' {
            self.parse_args(&mut end);
        }
        if let Some(global) = builtins::global(&name) {
            self.ctx.value = builtins::call_global(self.host, self.ctx, self.text, global);
            self.set_pos(end + 1);
            return Term::Function;
        }
        self.ctx.value = 0;
        let negative = self.at(self.pos()) == b'-';
        if negative {
            self.set_pos(self.pos() + 1);
        }
        let mut t = Term::None;
        while self.pos() < end && self.at(self.pos()).is_ascii_digit() {
            self.ctx.value = self
                .ctx
                .value
                .wrapping_mul(10)
                .wrapping_add(i32::from(self.at(self.pos()) - b'0'));
            self.set_pos(self.pos() + 1);
            t = Term::Value;
        }
        if negative {
            self.ctx.value = self.ctx.value.wrapping_neg();
        }
        self.set_pos(end + 1);
        t
    }

    /// `result` or a declared local.
    fn variable(&mut self, name: &[u8]) -> Option<Term> {
        if name == b"result" {
            return Some(Term::Result);
        }
        let i = self
            .ctx
            .locals
            .iter()
            .position(|l| l.name.bytes() == name)?;
        self.ctx.slot = i as i32;
        Some(Term::Local)
    }

    /// The run's `Role.` object, read from `Actor[0]` on first use.
    fn run_role(&mut self) -> bool {
        if self.ctx.run_role.is_none() {
            let Some(owner) = self.ctx.actor_at(0) else {
                return false;
            };
            let role = builtins::role_object(self.host, owner, self.ctx.role, 0);
            self.ctx.run_role = Some(role);
        }
        true
    }

    /// A member of an object: a method call (which may hand back an object
    /// to continue on) or a property read or assignment target.
    fn member(&mut self, object: Object) -> Term {
        self.ctx.args.clear();
        let start = self.pos();
        let mut i = start;
        let mut nlen = None;
        while i < self.len() {
            match self.at(i) {
                b'(' => {
                    nlen = Some(i - start);
                    self.parse_args(&mut i);
                    break;
                }
                b';' | b' ' | b'.' => {
                    nlen = Some(i - start);
                    break;
                }
                _ => i += 1,
            }
        }
        let name = self.slice(start, nlen.unwrap_or(i - start)).to_vec();
        self.set_pos(i + 1);
        match &object {
            Object::Actor => {
                if let Some(method) = builtins::actor_method(&name) {
                    self.ctx.status = 0;
                    self.ctx.value = builtins::call_actor(self.host, self.ctx, self.text, method);
                    return self.after_method();
                }
                if let Some(slot) = builtins::actor_property(&name) {
                    if self.ctx.reading {
                        self.ctx.status = 0;
                        // The class's property table is never filled: reads give 0.
                        self.ctx.value = 0;
                        return Term::Value;
                    }
                    self.ctx.pending = Some(Pending::ActorProperty);
                    self.ctx.slot = slot;
                    return Term::Property;
                }
                Term::None
            }
            Object::RunRole | Object::TemporaryRole(_) => {
                let role = match &object {
                    Object::TemporaryRole(r) => r,
                    _ => match self.ctx.run_role.as_ref() {
                        Some(r) => r,
                        None => return Term::None,
                    },
                };
                let Some(slot) = role.names.iter().position(|n| n.bytes() == name.as_slice())
                else {
                    return Term::None;
                };
                if self.ctx.reading {
                    self.ctx.status = 0;
                    self.ctx.value = role.values[slot];
                    return Term::Value;
                }
                self.ctx.slot = slot as i32;
                self.ctx.pending = Some(match object {
                    Object::TemporaryRole(r) => Pending::TemporaryRole(r),
                    _ => Pending::RunRole,
                });
                Term::Property
            }
        }
    }

    fn after_method(&mut self) -> Term {
        match self.ctx.status {
            1 => match self.ctx.chained.take() {
                Some(role) => self.member(Object::TemporaryRole(role)),
                None => Term::Value,
            },
            0xFFFF => {
                loop {
                    if self.pos() >= self.len() {
                        return Term::Value;
                    }
                    if matches!(self.at(self.pos()), b';' | b' ') {
                        break;
                    }
                    self.set_pos(self.pos() + 1);
                }
                self.set_pos(self.pos() + 1);
                Term::Value
            }
            _ => Term::Value,
        }
    }

    /// `Class[n].Member` read as text.
    fn object_expression(&mut self) -> Option<String> {
        self.skip_blanks();
        let start = self.pos();
        let mut end = start;
        while end < self.len() {
            if matches!(self.at(end), b' ' | b';' | b'.' | b'[' | b'(') {
                break;
            }
            end += 1;
        }
        let name = self.slice(start, end - start).to_vec();
        if self.at(end) == b'[' {
            self.ctx.actor = i32::from(self.at(end + 1)) - i32::from(b'0');
            end += 3;
        }
        if self.at(end) != b'.' {
            return None;
        }
        let mut out = None;
        if let Some(class) = builtins::class(&name) {
            end += 1;
            self.set_pos(end);
            self.ctx.args.clear();
            let mstart = end;
            let mut mlen = self.len() - mstart;
            while end < self.len() {
                match self.at(end) {
                    b'(' => {
                        mlen = end - mstart;
                        self.parse_args(&mut end);
                        break;
                    }
                    b';' | b' ' => {
                        mlen = end - mstart;
                        break;
                    }
                    _ => end += 1,
                }
            }
            let member = self.slice(mstart, mlen).to_vec();
            if let (Class::Actor, Some(m)) = (class, builtins::actor_text(&member)) {
                out = builtins::call_actor_text(self.host, self.ctx, m);
            }
        }
        self.set_pos(end + 1);
        out
    }
}

/// The text of argument `i` as a 16-byte name (longer arguments cut to
/// 16 bytes, a missing one empty).
pub(crate) fn arg_name<A>(ctx: &Context<A>, text: &[u8], i: usize) -> Name {
    match ctx.args.get(i) {
        Some(&(start, len)) => Name::from_bytes(arg_bytes(text, start, len)),
        None => Name::default(),
    }
}

pub(crate) fn arg_bytes(text: &[u8], start: i32, len: i32) -> &[u8] {
    let start = start.clamp(0, text.len() as i32) as usize;
    let end = (start + len.max(0) as usize).min(text.len());
    &text[start..end]
}

/// Argument `i` as an integer: a decimal (optionally negative), else the
/// value of the local it names (`-name` reads the local, not negated); 0
/// when missing.
pub(crate) fn arg_int<A>(ctx: &Context<A>, text: &[u8], i: usize) -> i32 {
    let Some(&(start, len)) = ctx.args.get(i) else {
        return 0;
    };
    let b = arg_bytes(text, start, len);
    let (lead, rest) = match b.first() {
        Some(b'-') => (b.get(1).copied().unwrap_or(0), &b[1..]),
        Some(&c) => (c, b),
        None => (0, b),
    };
    if lead.is_ascii_digit() {
        decimal(b)
    } else {
        ctx.locals
            .iter()
            .find(|l| l.name.bytes() == rest)
            .map_or(0, |l| l.value)
    }
}

/// Argument `i` as an id: `$hex`, else decimal.
pub(crate) fn arg_id<A>(ctx: &Context<A>, text: &[u8], i: usize) -> u32 {
    let Some(&(start, len)) = ctx.args.get(i) else {
        return 0;
    };
    let b = arg_bytes(text, start, len);
    match b.split_first() {
        Some((b'$', hex)) => hexadecimal(hex),
        _ => decimal(b) as u32,
    }
}

/// Argument `i` as `Actor` / `Actor[n]`: the actor in that slot. The name
/// is compared over the argument's own length, as the original does.
pub(crate) fn arg_actor<A: Copy + PartialEq>(ctx: &Context<A>, text: &[u8], i: usize) -> Option<A> {
    let &(start, mut len) = ctx.args.get(i)?;
    let b = arg_bytes(text, start, len);
    let mut slot = 0;
    if b.last() == Some(&b']') {
        slot = i32::from(b.get(b.len().wrapping_sub(2)).copied().unwrap_or(0)) - i32::from(b'0');
        len -= 3;
    }
    let len = usize::try_from(len).ok()?;
    if len > 5 || b.get(..len)? != &b"Actor"[..len] {
        return None;
    }
    ctx.actor_at(slot)
}

/// Optional minus, then decimal digits up to the first other byte.
pub(crate) fn decimal(b: &[u8]) -> i32 {
    let negative = b.first() == Some(&b'-');
    let mut v: i32 = 0;
    for &c in &b[usize::from(negative)..] {
        if !c.is_ascii_digit() {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add(i32::from(c - b'0'));
    }
    if negative { v.wrapping_neg() } else { v }
}

/// Hex digits (either case) up to the first other byte.
pub(crate) fn hexadecimal(b: &[u8]) -> u32 {
    let mut v: u32 = 0;
    for &c in b {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => break,
        };
        v = v.wrapping_mul(16).wrapping_add(u32::from(d));
    }
    v
}
