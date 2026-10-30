//! The three tiers of the object database and the original's routines
//! over them: id resolution, the string resolver, instancing, duplication,
//! promotion with its remap tables, removal and the template helpers.
//! Layout and evidence: `docs/formats/OTHER.md` (Object database).

use super::record::{section_range, set_section_u32, set_u32, u32_at};
use super::{
    CONTAINER, DOOR, ID_MASK, INHERIT, Item, LOCAL, MODEL, NAME, OPERATIVE, RANDOM_TABLE, Record,
    Store, TAG_MASK, WORLD,
};
use crate::Result;
use crate::objstrings::Library;

/// The tier an id selects: the level-local database (bit 31), the world
/// database (bit 30) or the base templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Base,
    Local,
    World,
}

impl Tier {
    /// The tier the original's id resolver picks: bit 31 first, then bit
    /// 30, otherwise the base tier.
    pub fn of(id: u32) -> Self {
        if id & LOCAL != 0 {
            Self::Local
        } else if id & WORLD != 0 {
            Self::World
        } else {
            Self::Base
        }
    }

    /// The id the resolver searches its tier for: the low 30 bits for the
    /// local and world tiers, the whole id for the base tier.
    pub fn index(id: u32) -> u32 {
        match Self::of(id) {
            Self::Base => id,
            _ => id & ID_MASK,
        }
    }

    /// The tag a tier adds to the ids it hands out.
    pub fn tag(self) -> u32 {
        match self {
            Self::Base => 0,
            Self::Local => LOCAL,
            Self::World => WORLD,
        }
    }
}

/// How deep promotion follows containers inside containers. The original
/// recurses without a limit (a container holding itself would exhaust its
/// stack); Exarust stops here.
const PROMOTION_DEPTH: usize = 64;

/// What a use of an object with an operative section does besides the use
/// test's screens: see [`ObjectDb::use_effect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseEffect {
    None,
    /// The object's header name goes to the registered reader (its item
    /// description).
    ReadHeaderName,
    /// The item container closes and the divination-card form opens.
    OpenDivinationCard,
}

/// The three tiers of the object database and their string libraries.
#[derive(Debug, Clone, Default)]
pub struct ObjectDb {
    /// The read-only templates of `objdb.rdb`.
    pub base: Store,
    /// A level's own objects (ids tagged [`LOCAL`]).
    pub local: Store,
    /// Objects promoted out of a level (ids tagged [`WORLD`]).
    pub world: Store,
    /// The shared library (`objstrings.rdb`), which is also the base
    /// tier's own.
    pub strings: Library,
    pub local_strings: Library,
    pub world_strings: Library,
    /// The id remap table: local ids a promotion moved and their world ids.
    remap: Vec<(u32, u32)>,
    /// The link remap table: the same for usable objects of kind 0, which
    /// door links follow.
    link_remap: Vec<(u32, u32)>,
    remap_on: bool,
    link_remap_on: bool,
}

/// A record taken out of a database ready to be put into another: its
/// payload, its table flags and the texts of the model's two own strings
/// (`None` for an id that is an inherit, a shared string, or own text the
/// source library has lost).
struct Copy {
    payload: Vec<u8>,
    flags: u32,
    texts: [Option<Vec<u8>>; 2],
}

/// Takes a record out of `db`'s `source` tier, resolving its own model
/// strings in `db`'s library; `None` for an id without database bits or a
/// record number and for one the tier lacks.
fn copy_source(db: &ObjectDb, source: Tier, id: u32) -> Option<Copy> {
    if id & ID_MASK == 0 || id & TAG_MASK == 0 {
        return None;
    }
    let at = db.store(source).find_index(id & ID_MASK)?;
    let view = db.store(source).view(at)?;
    let flags = db.store(source).entries()[at].flags;
    let payload = view.payload().to_vec();
    let mut texts = [None, None];
    if let Some(model) = section_range(&payload, MODEL) {
        for (i, field) in [0x14, 0x18].into_iter().enumerate() {
            let raw = u32_at(&payload[model.clone()], field);
            // Only an own-library id (bit 31 with a record number) is
            // remapped; an inherit and a shared id stay as they are.
            if raw <= INHERIT {
                continue;
            }
            texts[i] = db.library(source).bytes(raw & ID_MASK).map(<[u8]>::to_vec);
        }
    }
    Some(Copy {
        payload,
        flags,
        texts,
    })
}

/// Puts a taken record into `db`'s `target` tier under its first free id,
/// moving each own string into the tier's library (or an inherit when the
/// text is gone).
fn copy_into(db: &mut ObjectDb, target: Tier, copy: Copy) -> u32 {
    let Copy {
        mut payload,
        flags,
        texts,
    } = copy;
    if let Some(model) = section_range(&payload, MODEL) {
        for (i, field) in [0x14, 0x18].into_iter().enumerate() {
            let raw = u32_at(&payload[model.clone()], field);
            if raw <= INHERIT {
                continue;
            }
            let new = match &texts[i] {
                Some(text) => {
                    let slot = db.library_mut(target).alloc();
                    db.library_mut(target).set(slot, text);
                    slot | 0x8000_0000
                }
                None => INHERIT,
            };
            set_u32(&mut payload[model.clone()], field, new);
        }
    }
    let new = db.store(target).first_free_id();
    let (_, to) = db.store_mut(target).insert(new);
    db.store_mut(target).set_flags(to, flags);
    db.store_mut(target).store(to, &payload);
    new | target.tag()
}

impl ObjectDb {
    /// Reads `objdb.rdb` and `objstrings.rdb` into the base tier, as the
    /// original's lazy loader does (an empty `strings` gives an empty
    /// library, as a missing file does there).
    pub fn parse(db: &[u8], strings: &[u8]) -> Result<Self> {
        Ok(Self {
            base: Store::parse(db)?,
            strings: if strings.is_empty() {
                Library::default()
            } else {
                Library::parse(strings)?
            },
            ..Self::default()
        })
    }

    pub fn store(&self, tier: Tier) -> &Store {
        match tier {
            Tier::Base => &self.base,
            Tier::Local => &self.local,
            Tier::World => &self.world,
        }
    }

    pub fn store_mut(&mut self, tier: Tier) -> &mut Store {
        match tier {
            Tier::Base => &mut self.base,
            Tier::Local => &mut self.local,
            Tier::World => &mut self.world,
        }
    }

    /// The string library a tier's own strings live in (the shared one for
    /// the base tier).
    pub fn library(&self, tier: Tier) -> &Library {
        match tier {
            Tier::Base => &self.strings,
            Tier::Local => &self.local_strings,
            Tier::World => &self.world_strings,
        }
    }

    pub fn library_mut(&mut self, tier: Tier) -> &mut Library {
        match tier {
            Tier::Base => &mut self.strings,
            Tier::Local => &mut self.local_strings,
            Tier::World => &mut self.world_strings,
        }
    }

    /// The record an id names, by the original's id resolver.
    pub fn find(&self, id: u32) -> Option<Record<'_>> {
        self.store(Tier::of(id)).find(Tier::index(id))
    }

    /// The body of the first section with `tag` of the record `id` names.
    pub fn section(&self, id: u32, tag: u32) -> Option<&[u8]> {
        self.find(id)?.section(tag)
    }

    /// The original's flagged section lookup: the table flags must share a
    /// bit with `tag` before the chain is walked. It reads the local tier
    /// for bit 31 and the base tier for an untagged id, and gives nothing
    /// for a world id.
    pub fn flagged_section(&self, id: u32, tag: u32) -> Option<&[u8]> {
        let store = if id & LOCAL != 0 {
            &self.local
        } else if id & WORLD != 0 {
            return None;
        } else {
            &self.base
        };
        let at = store.find_index(id & ID_MASK)?;
        if store.entries()[at].flags & tag == 0 {
            return None;
        }
        store.view(at)?.section(tag)
    }

    /// Whether the record `id` names (0 names none) holds a section with
    /// `tag`, by walking its chain.
    pub fn has_section(&self, id: u32, tag: u32) -> bool {
        id != 0 && self.section(id, tag).is_some()
    }

    /// The display name of an object, by the original's string resolver:
    /// see [`Self::description`].
    pub fn name(&self, id: u32) -> Option<String> {
        self.resolve_string(id, 0x14)
    }

    /// The description of an object, by the original's string resolver:
    /// none for a missing record, one without a model section, or an id
    /// of 0; [`INHERIT`] reads the same field of the record its parent
    /// word names and looks it up in the shared library (one level only);
    /// an id with bit 31 set is the record's own tier library's
    /// (`& 0x3FFFFFFF`); any other id is the shared library's. Once an id
    /// is chosen the resolver succeeds even when the slot has no text,
    /// which reads as an empty string.
    pub fn description(&self, id: u32) -> Option<String> {
        self.resolve_string(id, 0x18)
    }

    fn resolve_string(&self, id: u32, field: usize) -> Option<String> {
        let tier = Tier::of(id);
        let record = self.store(tier).find(id & ID_MASK)?;
        let field_of = |r: &Record<'_>| r.section(MODEL).map_or(0, |m| u32_at(m, field));
        let raw = field_of(&record);
        let text = if raw == 0 {
            return None;
        } else if raw == INHERIT {
            let parent = self.find(record.parent_word())?;
            let raw = field_of(&parent);
            if raw == 0 {
                return None;
            }
            self.strings.get(raw)
        } else if raw & 0x8000_0000 == 0 {
            self.strings.get(raw)
        } else {
            self.library(tier).get(raw & ID_MASK)
        };
        Some(text.unwrap_or_default())
    }

    /// The smallest positive id the tier lacks.
    pub fn first_free_id(&self, tier: Tier) -> u32 {
        self.store(tier).first_free_id()
    }

    /// Inserts a new record under the tier's first free id with the given
    /// table flags and payload; returns the untagged id.
    fn insert_new(&mut self, tier: Tier, flags: u32, payload: &[u8]) -> u32 {
        let store = self.store_mut(tier);
        let id = store.first_free_id();
        let (_, at) = store.insert(id);
        store.set_flags(at, flags);
        store.store(at, payload);
        id
    }

    /// Clones the base template `id` into `tier` under the tier's first
    /// free id, as the original's template clone does: the table flags come
    /// along, the header names the template and both model string ids
    /// inherit. Returns the new tagged id, 0 for id 0, and the id itself
    /// when the base tier lacks it.
    pub fn clone_template_into(&mut self, id: u32, tier: Tier) -> u32 {
        if id == 0 {
            return 0;
        }
        let Some(at) = self.base.find_index(id) else {
            return id;
        };
        let Some(src) = self.base.view(at) else {
            return id;
        };
        let flags = self.base.entries()[at].flags;
        let mut payload = src.payload().to_vec();
        set_section_u32(&mut payload, NAME, 0x14, id);
        set_section_u32(&mut payload, MODEL, 0x14, INHERIT);
        set_section_u32(&mut payload, MODEL, 0x18, INHERIT);
        self.insert_new(tier, flags, &payload) | tier.tag()
    }

    /// Instantiates an untagged base record into `tier` when its header
    /// word (payload `0x18`) has bit 0, by [`Self::clone_template_into`];
    /// any other id comes back unchanged.
    pub fn instance_of(&mut self, id: u32, tier: Tier) -> u32 {
        if id & TAG_MASK == 0
            && self
                .base
                .find(id)
                .is_some_and(|r| r.header_word() & super::INSTANCE_ON_USE != 0)
        {
            return self.clone_template_into(id, tier);
        }
        id
    }

    /// The record's parent word (payload `0x1C`), or the id itself when it
    /// does not resolve.
    pub fn template_of(&self, id: u32) -> u32 {
        self.find(id).map_or(id, |r| r.parent_word())
    }

    /// The template an instance should name: the record's parent word when
    /// the base tier holds that id with a model section whose name is not
    /// empty, else the id itself.
    pub fn instance_template(&self, id: u32) -> u32 {
        let Some(record) = self.find(id) else {
            return id;
        };
        let parent = record.parent_word();
        let named = self
            .base
            .find(parent)
            .and_then(|r| r.section(MODEL).map(|m| m.first().copied().unwrap_or(0)))
            .is_some_and(|first| first != 0);
        if named { parent } else { id }
    }

    /// The bulk byte (model `0x1C`) of an object's template, as the
    /// original's helper reads it: a tagged id goes through its header's
    /// parent word, an untagged one reads its own model; 0 when there is
    /// none.
    pub fn template_bulk(&self, id: u32) -> u8 {
        let source = if id & TAG_MASK != 0 {
            self.section(id, NAME).map_or(0, |h| u32_at(h, 0x14))
        } else {
            id
        };
        self.section(source, MODEL)
            .and_then(|m| m.get(0x1C).copied())
            .unwrap_or(0)
    }

    /// The original's instancer for a placed or used object: a tagged id
    /// is migrated from its template and comes back unchanged; an untagged
    /// base record whose header word has bit 0 is copied into the local
    /// tier under the first free id, naming [`Self::instance_template`] in
    /// its header with both string ids inheriting and, when that template
    /// is another record, its occupancy, slot and coverage/protection
    /// blocks copied in. Any other id comes back unchanged.
    pub fn instantiate(&mut self, id: u32) -> u32 {
        if id & ID_MASK == 0 {
            return id;
        }
        if id & TAG_MASK != 0 {
            self.migrate(id);
            return id;
        }
        let Some(at) = self.base.find_index(id) else {
            return id;
        };
        let Some(src) = self.base.view(at) else {
            return id;
        };
        let marked = src
            .section(NAME)
            .is_some_and(|h| u32_at(h, 0x10) & super::INSTANCE_ON_USE != 0);
        if !marked {
            return id;
        }
        let source_description = src.section(MODEL).map(|m| u32_at(m, 0x18));
        let flags = self.base.entries()[at].flags;
        let mut payload = src.payload().to_vec();
        let template = self.instance_template(id);
        set_section_u32(&mut payload, NAME, 0x14, template);
        if let Some(model) = section_range(&payload, MODEL) {
            if id == template {
                set_u32(&mut payload[model.clone()], 0x14, INHERIT);
                set_u32(&mut payload[model], 0x18, INHERIT);
            } else {
                self.copy_body_blocks(&mut payload);
                let model = section_range(&payload, MODEL).expect("the model section");
                set_u32(&mut payload[model.clone()], 0x14, INHERIT);
                let description = u32_at(&payload[model.clone()], 0x18);
                // The original compares the copy's description with its
                // source's; they only differ if the body copy overwrote the
                // model, so this keeps an own string in practice never.
                let own = if source_description == Some(description) {
                    INHERIT
                } else {
                    match self
                        .strings
                        .bytes(description & ID_MASK)
                        .map(<[u8]>::to_vec)
                    {
                        Some(text) => {
                            let slot = self.local_strings.alloc();
                            self.local_strings.set(slot, &text);
                            slot | 0x8000_0000
                        }
                        None => INHERIT,
                    }
                };
                set_u32(&mut payload[model], 0x18, own);
            }
        }
        self.insert_new(Tier::Local, flags, &payload) | LOCAL
    }

    /// Copies the template's occupancy (`0x10`), slot table (`0xB0`) and
    /// coverage/protection (`0x150`) blocks into a payload's body: the
    /// template is the payload's parent word, resolved by id.
    fn copy_body_blocks(&self, payload: &mut [u8]) {
        let parent = u32_at(payload, 0x1C);
        let Some(body) = section_range(payload, super::BODY) else {
            return;
        };
        let Some(template) = self.section(parent, super::BODY) else {
            return;
        };
        let body = &mut payload[body];
        for (at, len) in [(0x10, 0xA0), (0xB0, 0xA0), (0x150, 0x50)] {
            super::record::copy_bytes(template, body, at, len);
        }
    }

    /// The original's local duplicator. An instance whose header flag bit
    /// 0 is clear clones the template its header names (when it names one)
    /// by [`Self::clone_template_into`]; one with the bit set is copied
    /// under the first free local id, its own strings gaining a reference
    /// each. Returns the new id, or the id itself when there is nothing to
    /// copy.
    pub fn duplicate_local(&mut self, id: u32) -> u32 {
        if id == 0 {
            return 0;
        }
        let Some(at) = self.local.find_index(id & ID_MASK) else {
            return id;
        };
        let Some(src) = self.local.view(at) else {
            return id;
        };
        let Some(header) = src.section(NAME) else {
            return id;
        };
        if u32_at(header, 0x10) & super::INSTANCE_ON_USE == 0 {
            let template = u32_at(header, 0x14);
            return if template != 0 {
                self.clone_template_into(template, Tier::Local)
            } else {
                id
            };
        }
        let payload = src.payload().to_vec();
        if let Some(model) = src.section(MODEL) {
            for field in [0x14, 0x18] {
                let raw = u32_at(model, field);
                if raw > INHERIT {
                    self.local_strings.add_ref(raw & ID_MASK);
                }
            }
        }
        let new = self.local.first_free_id();
        let (_, to) = self.local.insert(new);
        // The original reads the source's flags by its index after the
        // insertion, so a new id inserted at or before it takes the flags
        // of whatever entry now sits there (its own zero flags when it took
        // that very index).
        let flags = self.local.entries().get(at).map_or(0, |e| e.flags);
        self.local.set_flags(to, flags);
        self.local.store(to, &payload);
        new | LOCAL
    }

    /// The item dispatcher the game's item code uses: a local id is
    /// duplicated; a world id gives 0; an untagged id gives 0 when the base
    /// tier lacks it, a local instance when its header word has bit 0, and
    /// itself otherwise.
    pub fn instantiate_or_duplicate(&mut self, id: u32) -> u32 {
        if id & LOCAL != 0 {
            return self.duplicate_local(id);
        }
        if id & WORLD != 0 {
            return 0;
        }
        match self.base.find(id) {
            None => 0,
            Some(r) if r.header_word() & super::INSTANCE_ON_USE != 0 => {
                self.clone_template_into(id, Tier::Local)
            }
            Some(_) => id,
        }
    }

    /// Copies a record from one tier into another, as the character-data
    /// unit copies a record's item: the payload comes along, the table flags
    /// are the source's, and the model's name and description string ids
    /// whose own text the source tier holds are re-allocated in the target
    /// tier's library. An id with no database bits or no record number, and
    /// one whose record the source tier lacks, come back unchanged. The new
    /// id carries the target tier's tag.
    pub fn copy_record(&mut self, id: u32, source: Tier, target: Tier) -> u32 {
        match copy_source(self, source, id) {
            Some(copy) => copy_into(self, target, copy),
            None => id,
        }
    }

    /// [`Self::copy_record`] out of another database: a record of the
    /// other database's `source` tier (its payload, flags and own strings)
    /// is copied into this one under the first free id of `target`, with the
    /// same string rule. The prelude loader reads each entry's own database
    /// beside the live one this way. Ids the other database lacks, and ids
    /// without database bits or a record number, come back unchanged.
    pub fn copy_record_from(&mut self, id: u32, own: &ObjectDb, source: Tier, target: Tier) -> u32 {
        match copy_source(own, source, id) {
            Some(copy) => copy_into(self, target, copy),
            None => id,
        }
    }

    /// Copies an instance record into a tier, as the character-data unit
    /// copies an item into a save or a prelude database: only a record whose
    /// header word has the instance flag is copied. The instance is made
    /// first (a clone, or a duplicate of a local object), then its model's
    /// two string ids are re-made in the target library: an empty id
    /// becomes an inherit, an own id is read from the source tier's
    /// library, any other from the shared library, and the text is always
    /// allocated in the target library even when it reads as nothing (the
    /// original stores the id of the slot it then releases). The payload
    /// goes under the target's first free id with the source's flags and
    /// the returned id is tagged local, whatever the target tier.
    /// A missing record gives 0; a record without the flag comes back as it
    /// went in; an id the instance step leaves untagged gives 0.
    pub fn copy_instance(&mut self, id: u32, target: Tier) -> u32 {
        let Some(header) = self.section(id, NAME) else {
            return 0;
        };
        if u32_at(header, 0x10) & super::INSTANCE_ON_USE == 0 {
            return id;
        }
        let made = self.instantiate_or_duplicate(id);
        if made & TAG_MASK == 0 || made & ID_MASK == 0 {
            return 0;
        }
        let source = match made & TAG_MASK {
            WORLD => Tier::World,
            LOCAL => Tier::Local,
            _ => Tier::Base,
        };
        let Some(at) = self.store(source).find_index(made & ID_MASK) else {
            return 0;
        };
        let Some(view) = self.store(source).view(at) else {
            return 0;
        };
        let flags = self.store(source).entries()[at].flags;
        let mut payload = view.payload().to_vec();
        if let Some(model) = section_range(&payload, MODEL) {
            for field in [0x14, 0x18] {
                let raw = u32_at(&payload[model.clone()], field);
                let new = if raw & ID_MASK == 0 {
                    INHERIT
                } else {
                    let text = if raw & 0x8000_0000 != 0 {
                        self.library(source)
                            .bytes(raw & ID_MASK)
                            .map(<[u8]>::to_vec)
                    } else {
                        self.strings.bytes(raw & ID_MASK).map(<[u8]>::to_vec)
                    };
                    let slot = self.library_mut(target).alloc();
                    self.library_mut(target)
                        .set(slot, text.as_deref().unwrap_or_default());
                    slot | 0x8000_0000
                };
                set_u32(&mut payload[model.clone()], field, new);
            }
        }
        let new = self.store(target).first_free_id();
        let (_, to) = self.store_mut(target).insert(new);
        self.store_mut(target).set_flags(to, flags);
        self.store_mut(target).store(to, &payload);
        new | LOCAL
    }

    /// Promotes an object to the world tier, as the original does. A local
    /// object (tag exactly bit 31) comes back as its recorded world id when
    /// the id remap table holds it; otherwise its container children are
    /// promoted first and their ids rewritten, its own name and description
    /// strings move from the local library to the world library (an own
    /// string without text becomes [`INHERIT`]), it is inserted under the
    /// first free world id and deleted from the local tier, and the pair is
    /// recorded in the remap tables that are on (the link table only for an
    /// operative object of kind 0). A world object with a container
    /// promotes its children in place. Returns the world id, 0 for a
    /// missing local object, and any other id unchanged.
    pub fn promote(&mut self, id: u32) -> u32 {
        self.promote_at(id, 0)
    }

    fn promote_at(&mut self, id: u32, depth: usize) -> u32 {
        if depth > PROMOTION_DEPTH {
            return id;
        }
        match id & TAG_MASK {
            WORLD => {
                let Some(at) = self.world.find_index(id & ID_MASK) else {
                    return id;
                };
                if self.world.entries()[at].flags & CONTAINER == 0 {
                    return id;
                }
                let Some(src) = self.world.view(at) else {
                    return id;
                };
                let mut payload = src.payload().to_vec();
                self.promote_children(&mut payload, depth);
                if let Some(at) = self.world.find_index(id & ID_MASK) {
                    self.world.store(at, &payload);
                }
                id
            }
            LOCAL => {
                if self.remap_on
                    && let Some(&(_, world)) = self.remap.iter().find(|(local, _)| *local == id)
                {
                    return world;
                }
                let Some(at) = self.local.find_index(id & ID_MASK) else {
                    return 0;
                };
                let Some(src) = self.local.view(at) else {
                    return 0;
                };
                let mut payload = src.payload().to_vec();
                let had_children = section_range(&payload, CONTAINER).is_some();
                self.promote_children(&mut payload, depth);
                let at = if had_children {
                    match self.local.find_index(id & ID_MASK) {
                        Some(at) => at,
                        None => return 0,
                    }
                } else {
                    at
                };
                if let Some(model) = section_range(&payload, MODEL) {
                    for field in [0x14, 0x18] {
                        let raw = u32_at(&payload[model.clone()], field);
                        if raw <= INHERIT {
                            continue;
                        }
                        let new = match self.local_strings.bytes(raw & ID_MASK).map(<[u8]>::to_vec)
                        {
                            None => INHERIT,
                            Some(text) => {
                                self.local_strings.release(raw & ID_MASK);
                                let slot = self.world_strings.alloc();
                                self.world_strings.set(slot, &text);
                                slot | 0x8000_0000
                            }
                        };
                        set_u32(&mut payload[model.clone()], field, new);
                    }
                }
                let new = self.world.first_free_id();
                if self.remap_on {
                    self.remap.push((id, new | WORLD));
                }
                if self.link_remap_on
                    && Record::new(0, 0, &payload)
                        .section(OPERATIVE)
                        .and_then(|b| b.get(1))
                        .is_some_and(|&kind| kind == 0)
                {
                    self.link_remap.push((id, new | WORLD));
                }
                let flags = self.local.entries()[at].flags;
                let (_, to) = self.world.insert(new);
                self.world.set_flags(to, flags);
                self.world.store(to, &payload);
                self.local.remove_at(at);
                new | WORLD
            }
            _ => id,
        }
    }

    /// Promotes every child of a payload's container section (a count at
    /// body `0x14`, 12-byte entries from `0x18` whose first word is the
    /// child id) and writes the new ids back.
    fn promote_children(&mut self, payload: &mut [u8], depth: usize) {
        let Some(range) = section_range(payload, CONTAINER) else {
            return;
        };
        let count = u32_at(&payload[range.clone()], 0x14) as i32;
        for i in 0..count.max(0) as usize {
            let at = range.start + 0x18 + i * 12;
            if at + 4 > range.end {
                break;
            }
            let child = u32_at(payload, at);
            let promoted = self.promote_at(child, depth + 1);
            set_u32(payload, at, promoted);
        }
    }

    /// The world id a promoted local id became, while the id remap table
    /// is on.
    pub fn remapped(&self, id: u32) -> Option<u32> {
        self.remap
            .iter()
            .find(|(local, _)| *local == id)
            .map(|&(_, world)| world)
    }

    /// Turns the id remap table on or off; either empties it.
    pub fn set_remaps(&mut self, on: bool) {
        self.remap_on = on;
        self.remap.clear();
    }

    /// Turns the link remap table on or off; either empties it.
    pub fn set_link_remaps(&mut self, on: bool) {
        self.link_remap_on = on;
        self.link_remap.clear();
    }

    /// A scene entity's object id after the promotions recorded in the id
    /// remap table, as the original's entity rewrite maps it: an id tagged
    /// exactly local that the table holds becomes its world id; any other
    /// id is unchanged. The walk over the scene's entities is the caller's.
    pub fn remap_entity_id(&self, id: u32) -> u32 {
        if id & TAG_MASK != LOCAL {
            return id;
        }
        self.remapped(id).unwrap_or(id)
    }

    /// Rewrites the linked-object word (door body `+4`) of every local
    /// door through the link remap table, as the original does after a
    /// promotion pass, then turns the table off and empties it. Returns how
    /// many doors changed.
    pub fn apply_link_remaps(&mut self) -> usize {
        self.link_remap_on = false;
        let mut changed = 0;
        if !self.link_remap.is_empty() {
            for at in 0..self.local.len() {
                if self.local.entries()[at].flags & DOOR == 0 {
                    continue;
                }
                let Some(src) = self.local.view(at) else {
                    continue;
                };
                let Some(door) = section_range(src.payload(), DOOR) else {
                    continue;
                };
                let link = u32_at(&src.payload()[door.clone()], 4);
                let Some(&(_, world)) = self.link_remap.iter().find(|(local, _)| *local == link)
                else {
                    continue;
                };
                let mut payload = src.payload().to_vec();
                set_u32(&mut payload[door], 4, world);
                self.local.store(at, &payload);
                changed += 1;
            }
            self.set_link_remaps(false);
        }
        changed
    }

    /// What using an object gives, by its operative section, as the
    /// original's item-use test returns it (the codes' meanings in the item
    /// screen are not established): 0 without an operative section, for
    /// classes 2, 4 and 8, and for class 0, except that class 0 of kind
    /// `0x11` gives 2 when bits 16–23 of `word` (the caller's word) are
    /// clear; 1 for class 1 of kind 8, 9, 10 or 14 with those bits set; 2
    /// otherwise. For class 0 of another kind the original first runs its
    /// use action (kind 7 hands the header name to a registered reader,
    /// kind 5 opens a form), which is the caller's here.
    pub fn use_kind(&self, id: u32, word: u32) -> u8 {
        let Some(op) = self.section(id, OPERATIVE) else {
            return 0;
        };
        let class = op.first().copied().unwrap_or(0);
        let kind = op.get(1).copied().unwrap_or(0);
        let condition = word & 0xFF_0000 != 0;
        match class {
            2 | 4 | 8 => 0,
            0 if kind == 0x11 && !condition => 2,
            0 => 0,
            1 if matches!(kind, 8 | 9 | 10 | 14) && condition => 1,
            _ => 2,
        }
    }

    /// What using an object does besides the screens, as the original's
    /// class-0 use action runs it: only a class-0 object has one; kind 7
    /// hands its header name to the registered reader ([`UseEffect::
    /// ReadHeaderName`]) and kind 5 closes the item container and opens the
    /// divination-card form ([`UseEffect::OpenDivinationCard`]). Other kinds
    /// and classes do nothing. The original's reader callback and forms are
    /// the caller's.
    pub fn use_effect(&self, id: u32) -> UseEffect {
        let Some(op) = self.section(id, OPERATIVE) else {
            return UseEffect::None;
        };
        if op.first().copied().unwrap_or(0) != 0 {
            return UseEffect::None;
        }
        match op.get(1).copied().unwrap_or(0) {
            7 => UseEffect::ReadHeaderName,
            5 => UseEffect::OpenDivinationCard,
            _ => UseEffect::None,
        }
    }

    /// The original's guarded removal ("Remove DB reference"): a local or
    /// world object is deleted from its tier after releasing, in that
    /// tier's library, every model string id whose low 31 bits are not
    /// zero; an object with a random table section is kept; an untagged id is
    /// refused. Returns whether a record was deleted.
    pub fn remove(&mut self, id: u32) -> bool {
        let tier = if id & LOCAL != 0 {
            Tier::Local
        } else if id & WORLD != 0 {
            Tier::World
        } else {
            return false;
        };
        let Some(at) = self.store(tier).find_index(id & ID_MASK) else {
            return false;
        };
        if let Some(record) = self.store(tier).view(at) {
            if record.section(RANDOM_TABLE).is_some() {
                return false;
            }
            let ids = Self::model_string_ids(&record);
            for raw in ids {
                self.library_mut(tier).release(raw);
            }
        }
        self.store_mut(tier).remove_at(at);
        true
    }

    /// The original's unguarded deletion, for any tier: the record's model
    /// string ids whose low 31 bits are not zero are released in the tier's
    /// library (the shared one for the base tier) and the record deleted.
    pub fn delete(&mut self, id: u32) -> bool {
        let tier = Tier::of(id);
        let Some(at) = self.store(tier).find_index(id & ID_MASK) else {
            return false;
        };
        if let Some(record) = self.store(tier).view(at) {
            for raw in Self::model_string_ids(&record) {
                self.library_mut(tier).release(raw);
            }
        }
        self.store_mut(tier).remove_at(at);
        true
    }

    /// The string ids the removal routines release: each model id with its
    /// top bit cleared, when anything is left.
    fn model_string_ids(record: &Record<'_>) -> Vec<u32> {
        let Some(model) = record.section(MODEL) else {
            return Vec::new();
        };
        [0x14, 0x18]
            .into_iter()
            .map(|f| u32_at(model, f) & 0x7FFF_FFFF)
            .filter(|&raw| raw != 0)
            .collect()
    }

    /// Empties a tier, as the original's clear does; its string library is
    /// left as it is.
    pub fn clear(&mut self, tier: Tier) {
        self.store_mut(tier).clear();
    }

    /// Writes a tier as the original saves a database.
    pub fn write(&self, tier: Tier) -> Vec<u8> {
        self.store(tier).write()
    }

    /// The base tier decoded into items, in table order: what the game's
    /// callers read of `objdb.rdb`.
    pub fn items(&self) -> Vec<Item> {
        self.base.records().filter_map(|r| r.item().ok()).collect()
    }

    /// The first base record whose header name or display name matches
    /// `name`, ignoring case. An Exarust convenience: the original looks
    /// objects up by id only.
    pub fn find_by_name(&self, name: &str) -> Option<Record<'_>> {
        self.base.records().find(|r| {
            r.header()
                .is_some_and(|h| h.name.eq_ignore_ascii_case(name))
                || self
                    .name(r.id)
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
    }

    /// The first base record whose model name matches `model`, ignoring
    /// case.
    pub fn find_by_model(&self, model: &str) -> Option<Record<'_>> {
        self.base.records().find(|r| {
            r.model()
                .is_some_and(|m| m.name.eq_ignore_ascii_case(model))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::test_util::*;
    use super::*;
    use crate::objdb::{BODY, INSTANCE_ON_USE};

    fn put(store: &mut Store, id: u32, flags: u32, p: &[u8]) {
        let (_, at) = store.insert(id);
        store.set_flags(at, flags);
        store.store(at, p);
    }

    fn item(name: &str, flags: u32, parent: u32, name_id: u32, desc: u32) -> Vec<u8> {
        payload(&[
            (NAME, header(name, flags, parent)),
            (MODEL, model(name, 0, name_id, desc)),
        ])
    }

    /// Base 10 "Sword" (marked, shared strings 1 and 2) with a body; base
    /// 30 "Tpl" with a coverage of 50; base 20 marked, naming 30; base 12
    /// unmarked.
    fn db() -> ObjectDb {
        let mut db = ObjectDb::default();
        assert_eq!(db.strings.add(b"Sword"), 1);
        assert_eq!(db.strings.add(b"A sword."), 2);
        let mut body = vec![0u8; 0x224];
        body[0x1A6] = 4;
        let mut sword = item("Sword", INSTANCE_ON_USE, 0, 1, 2);
        sword.extend(section(BODY, &body));
        put(&mut db.base, 10, NAME | MODEL | BODY, &sword);
        put(&mut db.base, 12, NAME | MODEL, &item("Plain", 0, 0, 0, 0));
        let mut tpl_body = vec![0u8; 0x224];
        tpl_body[0x150] = 50;
        tpl_body[0x10] = 7;
        tpl_body[0x1A6] = 9;
        let mut tpl = item("Tpl", 0, 0, 1, 2);
        tpl[0x20 + 8 + 0x1C] = 6; // bulk
        tpl.extend(section(BODY, &tpl_body));
        put(&mut db.base, 30, NAME | MODEL | BODY, &tpl);
        let mut marked = item("Marked", INSTANCE_ON_USE, 30, 1, 2);
        marked.extend(section(BODY, &vec![0u8; 0x224]));
        put(&mut db.base, 20, NAME | MODEL | BODY, &marked);
        db
    }

    #[test]
    fn tiers_and_resolution() {
        assert_eq!(Tier::of(5), Tier::Base);
        assert_eq!(Tier::of(LOCAL | 5), Tier::Local);
        assert_eq!(Tier::of(WORLD | 5), Tier::World);
        assert_eq!(Tier::of(LOCAL | WORLD | 5), Tier::Local);
        assert_eq!((Tier::index(WORLD | 5), Tier::tag(Tier::World)), (5, WORLD));
        let mut db = db();
        assert_eq!(db.find(10).unwrap().header().unwrap().name, "Sword");
        assert!(db.find(LOCAL | 10).is_none());
        // The flagged lookup tests the table flags first and reads no world
        // id; the plain one walks the chain.
        put(&mut db.local, 3, NAME, &item("NoModelFlag", 0, 0, 0, 0));
        put(&mut db.world, 3, NAME | MODEL, &item("World", 0, 0, 0, 0));
        assert!(db.flagged_section(LOCAL | 3, MODEL).is_none());
        assert!(db.has_section(LOCAL | 3, MODEL));
        assert!(db.flagged_section(10, MODEL).is_some());
        assert!(db.flagged_section(WORLD | 3, MODEL).is_none());
        assert!(db.section(WORLD | 3, MODEL).is_some());
        assert!(!db.has_section(0, MODEL));
    }

    #[test]
    fn string_ids_follow_inherit_own_and_shared() {
        let mut db = db();
        assert_eq!(db.name(10).as_deref(), Some("Sword"));
        assert_eq!(db.description(10).as_deref(), Some("A sword."));
        let own = db.local_strings.add(b"Local name");
        put(
            &mut db.local,
            1,
            NAME | MODEL,
            &item("Inst", 0, 10, INHERIT, INHERIT),
        );
        put(
            &mut db.local,
            2,
            NAME | MODEL,
            &item("Own", 0, 10, own | 0x8000_0000, 0),
        );
        put(
            &mut db.local,
            3,
            NAME | MODEL,
            &item("Gone", 0, 0, 0x8000_0009, INHERIT),
        );
        // Inheriting reads the parent's field from the shared library.
        assert_eq!(db.name(LOCAL | 1).as_deref(), Some("Sword"));
        assert_eq!(db.description(LOCAL | 1).as_deref(), Some("A sword."));
        // An own string lives in the tier's library; an id of 0 is none.
        assert_eq!(db.name(LOCAL | 2).as_deref(), Some("Local name"));
        assert_eq!(db.description(LOCAL | 2), None);
        // A chosen id whose slot has no text still succeeds, empty; an
        // inherit whose parent (0) does not resolve fails.
        assert_eq!(db.name(LOCAL | 3).as_deref(), Some(""));
        assert_eq!(db.description(LOCAL | 3), None);
        // No model section, no record: none.
        put(
            &mut db.base,
            11,
            NAME,
            &payload(&[(NAME, header("F", 0, 0))]),
        );
        assert_eq!(db.name(11), None);
        assert_eq!(db.name(99), None);
    }

    #[test]
    fn instancing_and_cloning() {
        let mut db = db();
        // A record that is its own template: both ids inherit, the header
        // names it, the table flags come along.
        assert_eq!(db.instance_template(10), 10);
        let first = db.instantiate(10);
        assert_eq!(first, LOCAL | 1);
        let r = db.find(first).unwrap();
        assert_eq!(r.template(), 10);
        assert_eq!(r.flags, NAME | MODEL | BODY);
        let m = r.model().unwrap();
        assert_eq!((m.name_id, m.description_id), (INHERIT, INHERIT));
        assert_eq!(r.section(BODY).unwrap()[0x1A6], 4);
        // A record naming a template with a model: the header names the
        // template, the occupancy, slot and coverage blocks are copied (not
        // the bytes after them), and the description inherits too.
        assert_eq!(db.instance_template(20), 30);
        let second = db.instantiate(20);
        assert_eq!(second, LOCAL | 2);
        let r = db.find(second).unwrap();
        assert_eq!(r.template(), 30);
        let b = r.section(BODY).unwrap();
        assert_eq!((b[0x10], b[0x150], b[0x1A6]), (7, 50, 0));
        let m = r.model().unwrap();
        assert_eq!((m.name_id, m.description_id), (INHERIT, INHERIT));
        assert_eq!(db.name(second).as_deref(), Some("Sword"));
        // Unmarked, missing and empty ids come back as they are; a tagged
        // id is migrated and comes back.
        assert_eq!(db.instantiate(12), 12);
        assert_eq!(db.instantiate(99), 99);
        assert_eq!(db.instantiate(LOCAL), LOCAL);
        assert_eq!(db.instantiate(LOCAL | 1), LOCAL | 1);
        // Cloning a template into a tier.
        assert_eq!(db.clone_template_into(10, Tier::World), WORLD | 1);
        let r = db.find(WORLD | 1).unwrap();
        assert_eq!(r.template(), 10);
        assert_eq!(r.model().unwrap().name_id, INHERIT);
        assert_eq!(db.clone_template_into(99, Tier::World), 99);
        assert_eq!(db.clone_template_into(0, Tier::World), 0);
        // The marked-template helper.
        assert_eq!(db.instance_of(10, Tier::World), WORLD | 2);
        assert_eq!(db.instance_of(12, Tier::World), 12);
        assert_eq!(db.instance_of(LOCAL | 1, Tier::World), LOCAL | 1);
    }

    #[test]
    fn local_duplication() {
        let mut db = db();
        let own = db.local_strings.add(b"Mine");
        put(
            &mut db.local,
            1,
            NAME | MODEL,
            &item("Own", INSTANCE_ON_USE, 10, own | 0x8000_0000, INHERIT),
        );
        // A marked instance is copied whole; its own string gains a use.
        let copy = db.duplicate_local(LOCAL | 1);
        assert_eq!(copy, LOCAL | 2);
        assert_eq!(db.local_strings.refs(own), 2);
        let r = db.find(copy).unwrap();
        assert_eq!(r.flags, NAME | MODEL);
        assert_eq!(r.model().unwrap().name_id, own | 0x8000_0000);
        assert_eq!(db.name(copy).as_deref(), Some("Mine"));
        // The original reads the source's flags by its index after the
        // insertion: with ids 1 and 3, duplicating 3 inserts 2 at 3's index,
        // so the copy gets the new entry's own zero flags.
        db.local.remove(2);
        put(
            &mut db.local,
            3,
            NAME | MODEL,
            &item("Three", INSTANCE_ON_USE, 10, INHERIT, INHERIT),
        );
        let copy = db.duplicate_local(LOCAL | 3);
        assert_eq!(copy, LOCAL | 2);
        assert_eq!(db.find(copy).unwrap().flags, 0);
        // An unmarked instance clones the template its header names.
        put(
            &mut db.local,
            9,
            NAME | MODEL,
            &item("Plain", 0, 10, INHERIT, INHERIT),
        );
        let clone = db.duplicate_local(LOCAL | 9);
        assert_eq!(clone, LOCAL | 4);
        assert_eq!(db.find(clone).unwrap().header().unwrap().name, "Sword");
        // Nothing to copy: the id comes back.
        put(
            &mut db.local,
            8,
            NAME,
            &payload(&[(NAME, header("N", 0, 0))]),
        );
        assert_eq!(db.duplicate_local(LOCAL | 8), LOCAL | 8);
        assert_eq!(db.duplicate_local(LOCAL | 77), LOCAL | 77);
        assert_eq!(db.duplicate_local(0), 0);
        // The dispatcher.
        assert_eq!(db.instantiate_or_duplicate(10), LOCAL | 5);
        assert_eq!(db.instantiate_or_duplicate(12), 12);
        assert_eq!(db.instantiate_or_duplicate(99), 0);
        assert_eq!(db.instantiate_or_duplicate(WORLD | 1), 0);
        assert_eq!(db.instantiate_or_duplicate(LOCAL | 1), LOCAL | 6);
    }

    fn container(children: &[u32]) -> Vec<u8> {
        let mut b = vec![0u8; 0x18 + 12 * children.len()];
        b[0x14..0x18].copy_from_slice(&(children.len() as u32).to_le_bytes());
        for (i, c) in children.iter().enumerate() {
            b[0x18 + i * 12..0x1C + i * 12].copy_from_slice(&c.to_le_bytes());
        }
        b
    }

    #[test]
    fn promotion_moves_children_strings_and_ids() {
        let mut db = ObjectDb::default();
        let own = db.local_strings.add(b"Local name");
        put(
            &mut db.local,
            1,
            NAME | MODEL,
            &item("Child", 0, 0, own | 0x8000_0000, 0),
        );
        put(
            &mut db.local,
            2,
            NAME | CONTAINER,
            &payload(&[
                (NAME, header("Box", 0, 0)),
                (CONTAINER, container(&[LOCAL | 1])),
            ]),
        );
        db.set_remaps(true);
        // The child goes first and takes the first world id.
        assert_eq!(db.promote(LOCAL | 2), WORLD | 2);
        assert!(db.local.is_empty());
        assert_eq!(db.remapped(LOCAL | 1), Some(WORLD | 1));
        assert_eq!(db.remapped(LOCAL | 2), Some(WORLD | 2));
        let body = db.section(WORLD | 2, CONTAINER).unwrap();
        assert_eq!(u32_at(body, 0x18), WORLD | 1);
        assert_eq!(db.find(WORLD | 2).unwrap().flags, NAME | CONTAINER);
        // The own string moved to the world library.
        assert_eq!(db.name(WORLD | 1).as_deref(), Some("Local name"));
        assert_eq!(db.world_strings.refs(1), 1);
        assert_eq!(db.local_strings.refs(own), 0);
        // A recorded id comes back from the table.
        assert_eq!(db.promote(LOCAL | 1), WORLD | 1);
        assert_eq!(db.remap_entity_id(LOCAL | 1), WORLD | 1);
        assert_eq!(db.remap_entity_id(WORLD | 7), WORLD | 7);
        // Turning recording off empties the table.
        db.set_remaps(false);
        assert_eq!(db.remapped(LOCAL | 1), None);
        // A missing local id gives 0; untagged and both-tag ids come back.
        assert_eq!(db.promote(LOCAL | 1), 0);
        assert_eq!(db.promote(7), 7);
        assert_eq!(db.promote(LOCAL | WORLD | 1), LOCAL | WORLD | 1);
        // A world container refreshes its children in place.
        put(
            &mut db.local,
            5,
            NAME,
            &payload(&[(NAME, header("Coin", 0, 0))]),
        );
        let world_box = payload(&[
            (NAME, header("Box", 0, 0)),
            (CONTAINER, container(&[LOCAL | 5])),
        ]);
        put(&mut db.world, 9, NAME | CONTAINER, &world_box);
        assert_eq!(db.promote(WORLD | 9), WORLD | 9);
        assert_eq!(
            u32_at(db.section(WORLD | 9, CONTAINER).unwrap(), 0x18),
            WORLD | 3
        );
        // An own string without text becomes an inherit.
        put(
            &mut db.local,
            6,
            NAME | MODEL,
            &item("Lost", 0, 0, 0x8000_0007, 0),
        );
        let lost = db.promote(LOCAL | 6);
        assert_eq!(db.find(lost).unwrap().model().unwrap().name_id, INHERIT);
    }

    #[test]
    fn copying_a_record_remaps_only_its_own_strings() {
        let mut db = db();
        let own = db.local_strings.add(b"Local name");
        let desc = db.local_strings.add(b"A local blade.");
        put(
            &mut db.local,
            4,
            NAME | MODEL,
            &item("Blade", 0, 0, own | 0x8000_0000, desc | 0x8000_0000),
        );
        // A shared string id and an inherit are left alone.
        put(
            &mut db.local,
            5,
            NAME | MODEL,
            &item("Plain", 0, 0, 1, INHERIT),
        );
        // The copy takes the target tier's first free id and its tag, keeps
        // the source flags and gets its own text in the target library.
        let copied = db.copy_record(LOCAL | 4, Tier::Local, Tier::World);
        assert_eq!(copied, WORLD | 1);
        assert_eq!(db.find(WORLD | 1).unwrap().flags, NAME | MODEL);
        let model = db.find(WORLD | 1).unwrap().model().unwrap();
        assert_eq!(
            model.name_id & 0x8000_0000,
            0x8000_0000,
            "an own name stays own"
        );
        assert_eq!(
            db.world_strings
                .bytes(model.name_id & ID_MASK)
                .map(<[u8]>::to_vec)
                .as_deref(),
            Some(&b"Local name"[..])
        );
        assert_eq!(
            db.world_strings
                .bytes(model.description_id & ID_MASK)
                .map(<[u8]>::to_vec)
                .as_deref(),
            Some(&b"A local blade."[..])
        );
        // The source record and its strings are untouched.
        assert!(db.find(LOCAL | 4).is_some());
        assert_eq!(db.local_strings.refs(own), 1);
        // A shared id and an inherit come across unchanged.
        let plain = db.copy_record(LOCAL | 5, Tier::Local, Tier::World);
        let model = db.find(plain).unwrap().model().unwrap();
        assert_eq!((model.name_id, model.description_id), (1, INHERIT));
        // A record without a database bit, without a record number or
        // missing from the source tier comes back as it went in.
        assert_eq!(db.copy_record(7, Tier::Local, Tier::World), 7);
        assert_eq!(db.copy_record(LOCAL, Tier::Local, Tier::World), LOCAL);
        assert_eq!(
            db.copy_record(LOCAL | 99, Tier::Local, Tier::World),
            LOCAL | 99
        );
        // An own string whose text the source tier lost reads as inherit.
        put(
            &mut db.local,
            6,
            NAME | MODEL,
            &item("Lost", 0, 0, 0x8000_0009, 0),
        );
        let lost = db.copy_record(LOCAL | 6, Tier::Local, Tier::World);
        assert_eq!(db.find(lost).unwrap().model().unwrap().name_id, INHERIT);
        // The same tier as source and target copies into it.
        let again = db.copy_record(WORLD | 2, Tier::World, Tier::World);
        assert_eq!(again, WORLD | 4);
        assert!(db.find(WORLD | 2).is_some());
    }

    #[test]
    fn an_instance_copies_its_strings_into_the_target() {
        let mut db = db();
        let own = db.local_strings.add(b"Own name");
        put(
            &mut db.local,
            1,
            NAME | MODEL,
            &item("Own", INSTANCE_ON_USE, 0, own | 0x8000_0000, INHERIT),
        );
        // An own name comes from the local library, a shared id from the
        // shared one; the new ids name slots in the world library.
        let copied = db.copy_instance(LOCAL | 1, Tier::World);
        assert_eq!(copied, LOCAL | 1, "the returned id is tagged local");
        assert!(db.find(WORLD | 1).is_some());
        let model = db.find(WORLD | 1).unwrap().model().unwrap();
        assert_eq!(
            db.world_strings
                .bytes(model.name_id & ID_MASK)
                .map(<[u8]>::to_vec)
                .as_deref(),
            Some(&b"Own name"[..])
        );
        assert_eq!(model.description_id, INHERIT);
        assert_eq!(db.find(WORLD | 1).unwrap().flags, NAME | MODEL);
        // The instance step duplicated the local record, which shares its
        // own string (one more reference); the copy itself left it alone.
        assert_eq!(db.local_strings.refs(own), 2);
        // A marked base record is instantiated first (its model inherits
        // both template strings, so the copy names the shared text).
        let marked = db.copy_instance(10, Tier::World);
        assert_eq!(marked, LOCAL | 2);
        let model = db.find(WORLD | 2).unwrap().model().unwrap();
        // The instance names its template and inherits both of its strings.
        assert_eq!((model.name_id, model.description_id), (INHERIT, INHERIT));
        assert_eq!(db.name(WORLD | 2).as_deref(), Some("Sword"));
        assert_eq!(db.description(WORLD | 2).as_deref(), Some("A sword."));
        // A missing record gives 0; an unmarked record comes back as it is.
        assert_eq!(db.copy_instance(LOCAL | 99, Tier::World), 0);
        // An instance is made first even when it copies into its own tier:
        // the duplicate takes the first free id and the copy the one after
        // it, which is the id the routine returns.
        let first = db.local.first_free_id();
        assert_eq!(
            db.copy_instance(LOCAL | 1, Tier::Local),
            LOCAL | (first + 1)
        );
        assert!(db.find(LOCAL | first).is_some());
        assert!(db.find(LOCAL | (first + 1)).is_some());
        put(
            &mut db.local,
            2,
            NAME,
            &payload(&[(NAME, header("Plain", 0, 0))]),
        );
        assert_eq!(db.copy_instance(LOCAL | 2, Tier::World), LOCAL | 2);
        // A record without the instance flag comes back as it is, tagged or
        // not.
        assert_eq!(db.copy_instance(12, Tier::World), 12);
        // An own string the source tier lost still takes a slot, which is
        // released at once: the id points at an empty slot.
        put(
            &mut db.local,
            3,
            NAME | MODEL,
            &item("Lost", INSTANCE_ON_USE, 0, 0x8000_0009, INHERIT),
        );
        let lost = db.copy_instance(LOCAL | 3, Tier::World);
        let model = db.find(WORLD | lost & ID_MASK).unwrap().model().unwrap();
        assert_eq!(model.name_id & 0x8000_0000, 0x8000_0000);
        assert_eq!(db.world_strings.refs(model.name_id & ID_MASK), 0);
    }

    #[test]
    fn a_record_copies_out_of_another_database() {
        // The entry's own database: one item whose own string names it.
        let mut own = ObjectDb::default();
        let text = own.strings.add(b"Entry blade");
        put(
            &mut own.base,
            1,
            NAME | MODEL,
            &item("Blade", 0, 0, text | 0x8000_0000, INHERIT),
        );
        // A live local database with one record of its own.
        let mut live = ObjectDb::default();
        put(
            &mut live.local,
            1,
            NAME,
            &payload(&[(NAME, header("Live", 0, 0))]),
        );
        let copied = live.copy_record_from(LOCAL | 1, &own, Tier::Base, Tier::Local);
        assert_eq!(copied, LOCAL | 2, "the live tier's next free id");
        let model = live.find(copied).unwrap().model().unwrap();
        assert_eq!(
            live.local_strings
                .bytes(model.name_id & ID_MASK)
                .map(<[u8]>::to_vec)
                .as_deref(),
            Some(&b"Entry blade"[..])
        );
        // The entry database is only read.
        assert_eq!(own.base.len(), 1);
        assert_eq!(own.strings.refs(text), 1);
        // An id the other database lacks stays as it is.
        assert_eq!(
            live.copy_record_from(LOCAL | 9, &own, Tier::Base, Tier::Local),
            LOCAL | 9
        );
    }

    #[test]
    fn a_self_containing_box_cannot_recurse_forever() {
        let mut db = ObjectDb::default();
        put(
            &mut db.local,
            1,
            NAME | CONTAINER,
            &payload(&[
                (NAME, header("Box", 0, 0)),
                (CONTAINER, container(&[LOCAL | 1])),
            ]),
        );
        db.promote(LOCAL | 1);
    }

    #[test]
    fn door_links_follow_kind_zero_promotions() {
        let mut db = ObjectDb::default();
        let mut key = vec![0u8; 0x3C];
        key[0] = 1; // a key, kind 0
        put(
            &mut db.local,
            1,
            NAME | OPERATIVE,
            &payload(&[(NAME, header("Key", 0, 0)), (OPERATIVE, key)]),
        );
        let mut lever = vec![0u8; 0x3C];
        lever[0] = 3;
        lever[1] = 1; // kind 1: not recorded
        put(
            &mut db.local,
            2,
            NAME | OPERATIVE,
            &payload(&[(NAME, header("Lever", 0, 0)), (OPERATIVE, lever)]),
        );
        let mut door = vec![0u8; 20];
        door[4..8].copy_from_slice(&(LOCAL | 1).to_le_bytes());
        put(
            &mut db.local,
            3,
            NAME | DOOR,
            &payload(&[(NAME, header("Door", 0, 0)), (DOOR, door.clone())]),
        );
        door[4..8].copy_from_slice(&(LOCAL | 2).to_le_bytes());
        put(
            &mut db.local,
            4,
            NAME | DOOR,
            &payload(&[(NAME, header("Gate", 0, 0)), (DOOR, door)]),
        );
        db.set_link_remaps(true);
        assert_eq!(db.promote(LOCAL | 1), WORLD | 1);
        assert_eq!(db.promote(LOCAL | 2), WORLD | 2);
        assert_eq!(db.apply_link_remaps(), 1);
        assert_eq!(u32_at(db.section(LOCAL | 3, DOOR).unwrap(), 4), WORLD | 1);
        assert_eq!(u32_at(db.section(LOCAL | 4, DOOR).unwrap(), 4), LOCAL | 2);
        // The pass turned the table off and emptied it.
        assert_eq!(db.apply_link_remaps(), 0);
    }

    #[test]
    fn use_kinds() {
        let mut db = ObjectDb::default();
        for (id, class, kind) in [
            (1, 0, 0x11),
            (2, 0, 7),
            (3, 1, 9),
            (4, 1, 3),
            (5, 4, 0),
            (6, 3, 1),
        ] {
            let mut op = vec![0u8; 0x3C];
            op[0] = class;
            op[1] = kind;
            put(
                &mut db.base,
                id,
                NAME | OPERATIVE,
                &payload(&[(NAME, header("U", 0, 0)), (OPERATIVE, op)]),
            );
        }
        put(
            &mut db.base,
            7,
            NAME,
            &payload(&[(NAME, header("N", 0, 0))]),
        );
        let set = 0x0001_0000;
        assert_eq!((db.use_kind(1, 0), db.use_kind(1, set)), (2, 0));
        assert_eq!((db.use_kind(2, 0), db.use_kind(2, set)), (0, 0));
        assert_eq!((db.use_kind(3, 0), db.use_kind(3, set)), (2, 1));
        assert_eq!((db.use_kind(4, 0), db.use_kind(4, set)), (2, 2));
        assert_eq!((db.use_kind(5, set), db.use_kind(6, 0)), (0, 2));
        assert_eq!((db.use_kind(7, 0), db.use_kind(99, 0)), (0, 0));
    }

    #[test]
    fn class_zero_use_actions() {
        let mut db = ObjectDb::default();
        for (id, class, kind) in [(1, 0, 7), (2, 0, 5), (3, 0, 0x11), (4, 1, 7), (5, 1, 5)] {
            let mut op = vec![0u8; 0x3C];
            op[0] = class;
            op[1] = kind;
            put(
                &mut db.base,
                id,
                NAME | OPERATIVE,
                &payload(&[(NAME, header("U", 0, 0)), (OPERATIVE, op)]),
            );
        }
        assert_eq!(db.use_effect(1), UseEffect::ReadHeaderName);
        assert_eq!(db.use_effect(2), UseEffect::OpenDivinationCard);
        assert_eq!(db.use_effect(3), UseEffect::None);
        // Only class 0 has the action.
        assert_eq!(db.use_effect(4), UseEffect::None);
        assert_eq!(db.use_effect(5), UseEffect::None);
        // A record without an operative section does nothing.
        put(
            &mut db.base,
            6,
            NAME,
            &payload(&[(NAME, header("N", 0, 0))]),
        );
        assert_eq!(db.use_effect(6), UseEffect::None);
        assert_eq!(db.use_effect(99), UseEffect::None);
    }

    #[test]
    fn removal_and_deletion() {
        let mut db = db();
        let a = db.local_strings.add(b"A");
        let b = db.local_strings.add(b"B");
        // An own name and a plain id: the guarded removal releases both in
        // the local library (it masks bit 31 off whatever is left).
        put(
            &mut db.local,
            1,
            NAME | MODEL,
            &item("X", 0, 10, a | 0x8000_0000, b),
        );
        assert!(db.remove(LOCAL | 1));
        assert!(db.local.is_empty());
        assert_eq!((db.local_strings.refs(a), db.local_strings.refs(b)), (0, 0));
        assert!(!db.remove(LOCAL | 1));
        // An untagged id is refused; a random table is kept.
        assert!(!db.remove(10));
        put(
            &mut db.world,
            1,
            NAME | RANDOM_TABLE,
            &payload(&[(NAME, header("Table", 0, 0)), (RANDOM_TABLE, vec![0; 16])]),
        );
        assert!(!db.remove(WORLD | 1));
        assert!(db.find(WORLD | 1).is_some());
        // The unguarded deletion works on any tier: base 10 releases its
        // shared strings.
        assert_eq!(db.strings.refs(1), 1);
        assert!(db.delete(10));
        assert!(db.find(10).is_none());
        assert_eq!((db.strings.refs(1), db.strings.refs(2)), (0, 0));
        assert!(db.delete(WORLD | 1));
        assert!(!db.delete(WORLD | 1));
        // Clearing a tier leaves its library.
        put(
            &mut db.local,
            1,
            NAME,
            &payload(&[(NAME, header("Y", 0, 0))]),
        );
        let c = db.local_strings.add(b"C");
        db.clear(Tier::Local);
        assert!(db.local.is_empty());
        assert_eq!(db.local_strings.get(c).as_deref(), Some("C"));
    }

    #[test]
    fn template_helpers_and_bulk() {
        let mut db = db();
        put(
            &mut db.local,
            1,
            NAME | MODEL,
            &item("I", 0, 30, INHERIT, INHERIT),
        );
        assert_eq!(db.template_of(LOCAL | 1), 30);
        assert_eq!(db.template_of(LOCAL | 2), LOCAL | 2);
        assert_eq!(db.template_of(20), 30);
        // A local id reads its template's bulk, an untagged one its own.
        assert_eq!(db.template_bulk(LOCAL | 1), 6);
        assert_eq!(db.template_bulk(30), 6);
        assert_eq!(db.template_bulk(10), 0);
        assert_eq!(db.template_bulk(99), 0);
        // The parent must be a base record with a named model.
        assert_eq!(db.instance_template(LOCAL | 1), 30);
        put(&mut db.local, 2, NAME | MODEL, &item("J", 0, 12, 0, 0));
        assert_eq!(db.instance_template(LOCAL | 2), 12);
        let mut unnamed = item("", 0, 0, 0, 0);
        unnamed[0x20 + 8] = 0;
        put(&mut db.base, 40, NAME | MODEL, &unnamed);
        put(&mut db.local, 3, NAME | MODEL, &item("K", 0, 40, 0, 0));
        assert_eq!(db.instance_template(LOCAL | 3), LOCAL | 3);
    }

    #[test]
    fn a_changed_tier_writes_back_what_it_holds() {
        let mut db = db();
        db.instantiate(10);
        db.instantiate(20);
        db.remove(LOCAL | 1);
        let bytes = db.write(Tier::Local);
        let back = Store::parse(&bytes).unwrap();
        assert_eq!(back.write(), bytes);
        assert_eq!(back.len(), 1);
        assert_eq!(back.find(2).unwrap().template(), 30);
        let lookups = ObjectDb {
            base: db.base.clone(),
            ..ObjectDb::default()
        };
        assert_eq!(lookups.find_by_name("marked").unwrap().id, 20);
        assert_eq!(lookups.find_by_name("A sword"), None);
        assert_eq!(lookups.find_by_model("TPL").unwrap().id, 30);
        assert_eq!(lookups.items().len(), 4);
    }
}
