# Other formats

## Databases (`.rdb`)

Status: **framing** (`exa_formats::rdb`).

```
u32 magic       races 0xDBCB0D00, charroles 0xDBCB0D03, locales 0xDBAE0D01, objdb 0xDBAC0D09
u32 table_len   multiple of 16
records[table_len/16]: u32 id, u32 flags, u32 offset (from end of table), u32 size
payloads
```

`races.rdb` (18 records of 624 bytes), `charroles.rdb` (dialogue and AI
roles with their scripts and dialogue graphs; see [Roles](#roles-charrolesrdb)), `locales.rdb` (the level list:
ids 1 Exanima, 2 Exanima01 … 13 Exanima67, 17 Arak_Ar),
`objdb.rdb` (object database, 0x5900/16 = 1424 records), `objstrings.rdb`
(string library, magic `0xAFBF0C02`). Each table ends with one slot more
than it has records, which the original's writer saves as it holds it (see
[Object database](#object-database)); this reader skips entries pointing
outside the file. `objdb` item records, race records, role records and the locale
names are decoded; other payloads are not.

**Races** (`exa_formats::races`): 18 records of 624 bytes, then an empty
entry. Lengths below are as stored; the parser returns them in metres.
Layout:

| offset | field |
|---|---|
| `+0x00` | name[32] |
| `+0x18` | first flag word (u32, inside the name field after the short name): bit 0 bleeds; `0x8` in the story, a dead character of the race may be raised again (humans, the golem suit) [inf: from the character rule bit it sets]; `0x20` sets a character rule bit read by the damage routine [?]; `0x80` red regeneration (the golem suit); `0x200` hovering feet, so the simple ground probe (the wraith). Human `0x0F`, Ogre `0x27`, Sir, harvester and demon `0x23`, Fael, both ghouls, Gobbler and Manipulator `0x03` bleed; Skeleton and Ancient 0, Wraith `0x300`, golems `0x1030`, `0x2030`, the suit `0x20B8` do not. `0x2000` the body takes the AL power crystal (item 500) as its core and `0x1000` crystal A (item 431), each only while it wears none of the five power crystals (431, 451, 452, 499, 500; REA on the doll's drop rule, `exa_items::possession::takes_core`). Bits `0x2`, `0x4`, `0x10`, `0x100` not decoded |
| `+0x1C` | second flag word: `0x4` red regeneration (Ogre, Sir, the harvester, the demon: `0x7`, `0x7`, `0xF`, `0x7`), `0x8` quadruped, which stabs with both hands (the harvester); `0x1` and `0x2` set character rule bits whose effects are not traced. Fael, ghouls, Gobbler and Manipulator `0x3`; the others 0 |
| `+0x20` | body model stem[24]: `Skel` → `skelbase.rfc`, `GlmBsA` → `glmbsabase.rfc` |
| `+0x38` | shared moveset[16] (`Default`) |
| `+0x48` | the race's own moveset[16] (`Golem`, `Herath`, `Ogre`, `Demon A01`, …) |
| `+0x58` | kind word: `0x100` the fists are claws (weapon class `0xC0` instead of `0x80`: Ghoul and Ghoul H `0x100`; Sir, Fael, Gobbler and the harvester `0x20100`); the left hand's fist record is chosen by the whole word (`0x100` one claw record, `0x20100` the other, anything else a human fist), the right hand's by the race, so the harvester keeps a right fist. An item check matches `0x10000` (golems, `0x10001`) with items flagged `0x2000000` and `0x20000` (Ogre and the demon `0x20001`, the clawed Herath races) with items flagged `0x4000000`. Others `0x1` |
| `+0x5C` | clip-rate base: the character's clip clock advances by it times the per-motion factors each tick (1.0; the two ghouls 1.5) |
| `+0x60` | support-pin factor: scales the pull of the two thorax pins and the pelvis pin toward the stance (human 1.25) |
| `+0x64` | character size, × (1 − 0.05·height) at spawn (human 1.0, Ogre, Sir, Fael, the demon and Golem Boss 1.5, the harvester 1.8, the other golems but the suit 1.4): held weapons are placed and scaled by it |
| `+0x68` | body scale, capped at 1.5 then × (1 − 0.05·height) at spawn (human 1.0, Fael 1.35, Ogre and Sir 1.5, the golems but the suit 1.2, the harvester and the demon 1.8, so 1.5): the voice volume, turn and step rates follow it |
| `+0x6C` | 1.0 in every record, no reader found |
| `+0x70` | mass divisor: the spawn's part-mass normalisation multiplies its divisor by it, so the parts are lighter (1.0; Sir, Fael and the demon 1.1) |
| `+0x74` | balance bonus (1.0 in every record): the balance score gains 0.05 × it, and the recovery-step reach grows by (1 + it); the original sets 1.5 or 2.0 instead for some creature types |
| `+0x78` | stance factor: the share of muscle tone a standing body uses (tone target = tone × it) and the divisor of the standing joint-muscle damping, k = 40 / it (human 0.5; skeleton, fael and ghouls 0.75; golems and the harvester 1.0) |
| `+0x7C` | arm relaxation: scales how far every part's link-muscle anchors may relax toward the current pose while the clip holds still, and the arms' relaxed stiffness, 0.4·(0.5 + 0.5·it) (human 1.0, skeleton 0.5) |
| `+0x80` | limb damping ratio r: a living body's velocity retention is scaled by u², u = (1 − balance)(1 − r) + r (1.0 for most races, so no extra damping; golems 0.95, the golem suit 0.97) |
| `+0x84` | unarmed reach, cm: the character's reach while it holds no weapon (human, skeleton, ancient, Manipulator, Wraith, the golem suit 50; Ogre, the ghouls, the demon and the golems 100; Fael 130; Sir, Gobbler and the harvester 150) |
| `+0x88` | swing-turn factor: scales each held weapon's swing-turn amplitude, how far the AI turns its body through a swing (1; Ogre, Gobbler and the golem suit 0.5; Sir, the harvester, the demon and the other golems 0) |
| `+0x8C` | AI attack bonus: the AI's combat state scales an attack-opportunity term by (1 + it) (Fael 0.2, the others 0) [inf: one reader] |
| `+0x90` | recovery factor: scales each held weapon's recovery, the frames the stance blends in over after an attack, rounded (1; golems 0.5) |
| `+0x94` | gait table: 4 rows (walk, jog, sprint, backpedal) × 5 floats, see below |
| `+0xE4` | health factor (damage model, below) |
| `+0xE8`–`+0x108` | bytes and floats read by view and camera routines, not decoded (Sir `+0xF8`/`+0xFC` −8 and 14, `+0x100`–`+0x108` 1.4) |
| `+0x110` | perception: how well the race's AI sees a character masked by a mind power; p scales its sighting range (1200 cm) by 1 − 0.9·(1 − p), and its outer range is that range scaled again by 1 − 0.8·(1 − p) (1; Ancient 0.5; Skeleton and Wraith 0). A feral character uses 1 − feral/2 |
| `+0x11C` | protection (natural armour, below) |
| `+0x12C` | held-item offset: three signed bytes × 0.1 cm added to the grip of items in the hands, x taken off in the right hand and added in the left (the demon 90, so 9 cm along x; 0 elsewhere) |
| `+0x134`, `+0x154`, `+0x164` | sound names (`Golem_Foley`; `FlsCol`, `SkelHit`, `HitGlm`; `WndFls`, `SkelHit`, `HitGlm`), readers not traced |
| `+0x144` | footstep sound set (up to 15 characters): `StpSft` (Human, Ancient, the ghouls, Gobbler, Manipulator, the harvester, Wraith), `SkelStp`, `StpHvy` (Ogre, Sir, the demon), `StpMed` (Fael), `Golem_Stp`. Characters whose record flags them as golems use `Golem_Stp` whatever their race |
| `+0x174`, `+0x184` | voice set names (`VH{S}`, `VH{S}DA`, `VOM01`, …; `{S}` becomes `m` or `f`), read by the race loader, not decoded |
| `+0x1B4` | voice volume: × the body scale at spawn (1; Ogre, Sir and the demon 3; Fael, Ghoul and the harvester 1.5; Ghoul H and Manipulator 2; golems 0.5) |
| `+0x1C4` | attack voices (i32): 0 never voices an attack; otherwise a swing voices with odds of one (or two) in a counter that grows by (this − 1) with each voice and shrinks by 1 with each silent swing [inf] (human, Ancient, Wraith 4; Gobbler, Manipulator 3; Fael, ghouls, the harvester, the demon 2; Ogre, Sir 1; skeletons and golems 0). A feral character uses 2 |
| `+0x21C` | coverage (natural armour, below); the rest of the record not decoded |

Gait rows, per second as stored. The original's loader divides columns 0
and 3 by 83 and column 1 by 83² to get per-tick values, and negates the
backpedal row's columns 0 and 3.

| column | meaning |
|---|---|
| 0 | thrust: the push along the heading (human walk 88.9, jog 350, sprint 472.2, backpedal 77.8) |
| 1 | push scale: how the push enters the body's carry (550 in every human row; 1600 in some creatures' walk) |
| 2 | never read (0 in every record but the backpedal row, which holds stray bytes `84 1E 00 00`): the loader fills this slot with a built-in value per row (0.05, 0.2, 0.25, 0.05), which the locomotion reads × 1000 as the brake distance in cm: a walking body brakes 50 cm from its goal, jogging 200, sprinting 250, backpedalling 50 (a running body never brakes for its goal). The parser returns these as `Gait::brake_distance` (0.5, 2, 2.5, 0.5 m), the fallback's rows included |
| 3 | clip rate, frames per second (human 27, 16.6, 13.5, 28): the base rate of the locomotion clock, blended from walk toward the gait by the run blend |
| 4 | speed along the heading under which the gait's phase push is boosted (0 everywhere but the harvester's walk, 20) |

A feral character (character record feral level f > 0) sprints with the
sprint row moved toward the jog row by f, every column alike, set at its
spawn (`RaceBody::sprint_gait`): a fully feral zombie sprints at the jog
row.

How the fields were established: reverse engineering of `Exanima.exe`
0.9.5. The character builder looks the race up by its id (race byte + 1) in
the `races.rdb` id database, which is loaded raw, so every read of a race
field uses the file's own record and offsets. Each field's meaning comes
from the routine that reads it: the character builder (scale capped at
1.5, size, the mass divisor, the height factor), the character init (the
clip-rate base as (83/83) × the field, the flag words to the character's
rule bits, perception, the AI bonus, voice volume and attack voices), the
weapon pass on every equip (recovery, swing turn, unarmed reach), the
moveset chooser and fist record choice (claws), the footstep set, the hand
placement, the AI's sighting ranges and attack scoring, the support pins,
the balance score and recovery steps, the stance factor of the body tick,
the arms' anchor relaxation and the limbs' retention. When the id is
missing, the original uses a built-in record instead, through the same
loader: every factor 1.0, an all-zero gait table (so a character on it
cannot walk) with the brake distances, 100 cm of unarmed reach, swing turn
and recovery 1, attack bonus 0, perception 1, no footsteps, voice volume 0
and attack voices 0 (`RaceBody::FALLBACK`, `RaceStats::FALLBACK`). The
character builder copies the first flag word to the character; the damage
routine's blood branch tests bit 0, and the race loader turns the blood
dirt mode on for the race's body-model materials whose names start with
`s` when it is set ([RENDERING.md](../RENDERING.md), Blood). The values
per race are printed by `exa-tool races`; a corpus test checks every
field of every record.

A character record's race byte is the race's id minus one. Placed
characters across the levels, by race byte:

- 0 Human 103, 1 Skeleton 147, 2 Ancient 23, 4 Sir (Herath ogre) 1,
  5 Fael 6, 6 Ghoul 56, 7 Ghoul H 41, 8 Gobbler 10, 9 Manipulator 19,
  10 Harvester 3.
- 16 Golem Guard 46, 17 Golem Boss 1, 18 Golem Suit 1, 19 Golem BAM 10,
  20 Golem BAF 10.

Bodies of the human frame (human, skeleton, ancient) name their parts like
the human body (`01Pelvis`). The others name them like apparel
(`0100Pelvis`, pelvis at the origin). All use the bone indices of
`humanoid.rsq` and its clips. Root positions are scaled by the body's leg
length against `humanbase` (golem 1.42×, ogre 1.39×).

**Roles** (`exa_formats::roles`): `charroles.rdb`; see
[Roles](#roles-charrolesrdb) below. The rules that read the table (role
lists, relations, the attitude, encounters) are in
[`crates/exa_ai/README.md`](../../crates/exa_ai/README.md).

**Locales** (`locales.rdb`, magic `0xDBAE0D01`): 17 records of 44 bytes,
a name field of 20 bytes (`Exanima`, `Exanima01` … `Exanima67_C1`,
`Arak_Ar`), then a word that is 1 for every level but `Exanima`, the rest
zero. The original's lookup compares and copies only the first 16 bytes of
the field. A level's locale is the id of the record whose name is the
level's without its extension (`exanima01.rfc` → 2); the lookup compares
the 16 bytes raw against the stored name lower-cased (Exarust lower-cases
the query too, so case is ignored); entries for one character compare their
locale with it [guess: the global those entries read is the current
locale's id]. `exa_formats::locales::LocaleDb` loads it and copies the
16-byte name; `exa_formats::uid` is the mutable id-keyed database whose
loader checks the expected magic and runs an upgrade hook before clearing
the table ("Invalid or obsolete database format."), with find-by-id,
record views, in-place payload replacement, and the separate sorted
id→offset index (insert, find, delete and block store; a block's first
word is its length, itself included; the game does not read it yet).

## Object database

Status: **verified** for the file framing and its writer, the upgrade of the
`0xDBAC0D08` and `0xDBAC0D07` layouts, the record chain, the three tiers, the
string library and its writer, the sections below, the random-table condition word,
instancing, duplication, promotion, removal, template migration with the
condition refresh, the item-use test, and the packed fields and amounts
(`exa_formats::objdb`, `exa_formats::objstrings`). Not reproduced: the body
converter of the oldest layout (`0xDBAC0D06`, not shipped), the item factory's
regeneration of `@` weapons during migration, the editor's palette
randomiser, and the scene walks of the entity lookup and entity-id rewrite
(the id rule itself is `ObjectDb::remap_entity_id`). The item fields
themselves, their apparel meaning and their readers are in
[Object database items and apparel painting](#object-database-items-and-apparel-painting)
below.

### Framing, the table and the writer

```
u32 magic       0xDBAC0D09 (older: 0xDBAC0D08, 0xDBAC0D07, 0xDBAC0D06)
u32 table_len
entries[table_len/16 - 1]: u32 id, u32 flags, u32 offset, u32 size
u8  slot[16]    the table slot after the last entry
payloads        the blob; offsets count from the end of the table
```

The loader counts `table_len/16 − 1` records (the table length is read as an
unsigned word shifted down, then made a signed count). With fewer than one
record it reads nothing past the header: the writer saves an empty database as
the 8-byte header alone (a one-slot table length and no table), which is how
half the level string libraries ship. Otherwise it keeps the table and the blob as
the file has them: it does not sort the table, compact the blob or check the
offsets. Lookups are a binary search that compares ids by their 32-bit
difference; insertion uses its own search of the same kind, and a duplicate
id is refused. A record whose size is 0 has no view (lookups that need one
fail).

Storing a payload of the same size overwrites it in place; any other size
first removes the old bytes from the blob (every entry whose payload starts
after them moves down by the old size), then appends the new payload at the
blob's end. Deleting stores an empty payload and closes the table up; the
table's memory is not shrunk, so the slot after the last entry then holds the
deleted entry (when it was the last) or a copy of the new last entry. After
the table grows, that slot lies past the allocation.

The writer saves the magic, a table length of `(records + 1) × 16`, and, when
there is a record, the table, the slot after it and the blob as held. So a
database the original loaded and saves unchanged is written back byte for
byte, and the shipped files show the editor's history: `objdb.rdb`'s blob has
371 payloads out of table order and 80 bytes of holes, and the slots after
the tables hold stale entries (`arenao3`: an entry for id `0x47`) or heap
bytes (`0xBAADF00D` in `exanima05`'s library). Exarust keeps all of it and
writes it back; it writes zeros for the slot once the table has grown.

**Older layouts.** After loading, a magic other than `0xDBAC0D09` is looked up
in a three-entry table and runs that converter and every newer one: `0D08`
the operative converter; `0D07` the weapon converter, then the operative one;
`0D06` the body converter, then both. Any other magic logs "Invalid or
obsolete database format." and empties the database (Exarust: an error).
Each converter rewrites one section in every record: it gets the old section
and a zeroed new body, the old section is deleted and the new body appended
with the same tag. 17 of the 20 shipped arena actors' databases
(`actors.rpk`) are `0D08`; none holds an operative section.

- Operative (`0D08`, 60-byte body): the old class byte maps through
  `[0, 1, 4, 3, 5, 6, 7, 8, 2, 0, 0, 0]`; the kind maps through
  `[0, 8, 11, 7, 1, 12, 2, 13, 6, 5, 14, 9, 10, 3, 4, 0]` (a higher kind
  reads on into the class table) unless the new class is a relay (7), which
  keeps it; bytes 2–31 are kept, the rest is zero.
- Weapon (`0D07`, 96-byte body): the flags from the old word at `0x10`
  (bits 16–23: 0 gives `0x11`, `0x14` for class byte `0x1F` = 4, `0x13` for
  5–6, else `0x12`; `0x10000` gives `0x421` for class 0, `0x23` for 5–6, else
  `0x22`; `0x20000` `0x32`; `0x410000` `0x521`; else `0x22`), with `0x400`
  and `0x100` adding `0x1000000` and `0x2000000`; the seeds from `0` and `4`
  to `4` and `0xC`; the stat bytes `0x14`–`0x16`, `0x1B`, `0x17`, `0x1F`,
  `0x1E` to `0x30`–`0x33`, `0x40`–`0x42`; byte `0x34` = 1; `0x35`/`0x36` from
  byte `0x1C` = v (below `0x47`: 20 and 200; below `0x5A`: 2(v − 10) and
  2(v + 10); else 2(2v − 100) and 200, all in bytes); `0x18`–`0x1A` to
  `0x37`–`0x39`; the words at `0x20` and `0x24` to `0x44` and `0x48`.

### Records, sections and flags

A record payload is a chain of `u32 tag, u32 size` sections, the size
counting the 8-byte header. The original's section finder walks it without
checks (a size of 0 would loop); Exarust stops at a size below 8 or past the
payload. The table's **flags word is the OR of the tags** of the sections a
record holds: all 1423 records of `objdb.rdb` but "Map", whose own
`0x80000b03` section is not in its flags. Only the flagged lookup tests the
flags (it reads the local tier for a local id, the base tier for an untagged
one, and nothing for a world id); the plain lookup and the existence test
walk the chain. Every shipped record starts with its header, and several
routines read the header's words straight from payload `0x18` (flags) and
`0x1C` (parent) rather than through the finder.

| tag | section |
|---|---|
| `0x10000000` | header: display name[16], flags word at `0x10`, parent at `0x14` |
| `0x00100000` | model: ground/icon model[16], flags at `0x10`, name string id at `0x14`, description string id at `0x18`, bulk byte `0x1C`, rarity `0x1D`, value word `0x1E` (compact amount), size words at `0x20`/`0x24` |
| `0x1000` | item body (548 bytes; worn model, occupancy `0x10`, slots `0xA8`, coverage `0x150`, protection `0x1A0`, appearance `0x1B0`, special `0x214`, power `0x218`, set `0x220`) |
| `0x200` | random table (see [Containers and random tables](#containers-and-random-tables); its rows' condition word below) |
| `0x4000` | weapon (flags `0`, seeds `4`/`8`, quality/damage/dirt `0xC`–`0xE`, materials `0x10`, stored choices `0x50`, stats `0x30`–`0x42`) |
| `0x8000` | shield (weight/block/bulk at body 4/5/6) |
| `0x10000` | operative object (class `0`, kind `1`, flags `2`, value `4`, time `6`, target `8`, second/code `12`) |
| `0x20000` | container: count at `0x14`, 12-byte child entries from `0x18` (the child's id, then two words), so a body is `0x18 + 12n` bytes (all 1283 shipped containers) |
| `0x40000` | door/link: flags at `0`, linked object at `4` (see [Door and operative sections](#door-and-operative-sections)) |
| `0x100` | the torch's extra section; 16 zero bytes in the one shipped record, meaning not established |
| `0x80000b03` | the map record's own section (record "Map"): zeroed volume, then `Map_Exanima01` |
| `0x8000ac07` | volume: three 16-bit extents, a zeroed word, flags at body `8`, entrance at `0x0c`, the level it leads back from at `0x10`. Shipped in every level's own item records as its transition markers (54 across the levels, all flags `0x1040`/`0x40`); flags' low byte `0x40` makes it a marker (`exa_formats::objdb::Volume`) |

The original's section editing routines (used by the converters): delete
removes a section and closes the gap; resize to a smaller or equal body
writes the new size and closes the gap, while growing only moves the bytes
after the section up and leaves the size word and the gap as they were;
append adds a section at the end (its body uninitialised there, zeroed in
Exarust); a mask test asks whether any tag shares a bit with a mask.

**Header word** bit 0 marks a template the game instantiates into the local
tier when it is used; bits 16–23 are the material class that template
migration compares. The parent word is the editor folder for a global item
(record 8 "Braies_A" names folder 91 "+Misc"), the source item for a
level-local copy, and the template id for an instance.

**Model flags** bit 0 dims the model's voxel lights and glow meshes by the
placed copy's condition byte (see `Item::light_share`); bits 4–6 are the
shop's rarity tier; the two words at `0x20`/`0x24` must be non-zero for a
stocked item [`guess`: the icon's size]. The name and description ids follow
the string rules below.

### Tiers, ids and lookup

Three databases exist: the base templates (`objdb.rdb`, no id tag, its string
library the shared `objstrings.rdb`), a level's local objects (ids tagged
`0x80000000`, its own library), and the world tier (ids tagged `0x40000000`,
its own library) that objects are promoted into. The id resolver checks bit
31 first (local), then bit 30 (world), otherwise the base tier; the low 30
bits are the record id, and the base lookup passes the id unmasked. The first
free id is the smallest positive integer the tier lacks: the original scans
the table, stepping past matching ids and stopping at the first id above the
candidate. Clearing a tier empties its table and blob and leaves its string
library alone.

### String library

`objstrings.rdb`, and each level's `0xDBCD` sub-chunk, is a pool of display
names and description phrases:

```
u32 magic       0xAFBF0C02
u32 table_len
entries[table_len/16 - 1]: u32 unused, u32 references, u32 offset, u32 length
u8  slot[16]    the table slot after the last entry
text            from the end of the table, Latin-1
```

It is loaded and saved like the record databases (the same count rule, the
header alone for no entries, the slot and the text written back as held);
offsets are not checked. Ids are one-based; the getter fails for id 0 and an
entry of length 0. The first word of an entry is never read (0 in every
shipped entry, zeroed when a slot is freed). The allocator takes the first
slot without references at or after the free hint (giving it one reference,
its offset and length left as they are), else appends a zeroed slot with one
reference. The setter stores text: empty text releases the slot and fails;
text of the slot's length overwrites it in place; any other length removes
the old text (every entry whose text starts after it moves down) and appends
the new text at the end. Releasing decrements the reference count (a 32-bit
count, so a free slot wraps instead) and, at zero, zeroes the slot, leaving
its text as a hole and raising the free hint to the slot when it is higher.
The shipped `objstrings.rdb` has 1124 entries and 460 bytes of holes; 11 of
the 22 level libraries are the header alone.

A record's model section names its display name and description by string id,
resolved by the original's string resolver (the record found in the tier its
id selects, by the low 30 bits):

| id | meaning |
|---|---|
| 0 | none |
| exactly `0x80000000` | inherit: read the same field of the record the payload's parent word (`0x1C`) names and look it up in the **shared** library (one level; a parent that does not resolve, or a field of 0, gives none) |
| bit 31 set with other bits | the record's own tier library, id `& 0x3FFFFFFF` |
| anything else | the shared library |

Once an id is chosen the resolver succeeds even when the slot has no text
(an empty name). The game's item names and description phrases are these
strings (record 2's model ids resolve to "Leather Gloves" and "A pair of
thick leather gauntlets."); the 119 own strings of the level databases all
resolve in their level's library.

### Instancing, duplication, promotion and removal

- **Template clone**: a base record is copied into a tier under its first
  free id with the base entry's table flags; the header's parent becomes the
  template's id and both model string ids inherit. A missing id comes back
  unchanged, 0 gives 0.
- **Instancing** (a placed or used object): a tagged id is migrated and comes
  back unchanged. An untagged base record whose header flags have bit 0 is
  copied into the local tier under its first free id; its header names the
  *instance template*: the record's parent word when the base tier has that
  id with a model section whose name is not empty, else the record itself.
  Both model ids inherit. When the template is another record, the copy's
  body takes the template's occupancy (`0x10`, 160 bytes), slot table
  (`0xB0`, 160 bytes) and coverage/protection (`0x150`, 80 bytes). (The
  original then compares the copy's description id with its source's to
  decide whether to make it an own string; the two never differ, so the
  description always inherits.) Unmarked or missing records come back
  unchanged.
- **The item dispatcher**: a local id is duplicated; a world id gives 0; an
  untagged id gives 0 when missing, a template clone into the local tier
  when its header word has bit 0, and itself otherwise.
- **Local duplication**: an instance whose header flag bit 0 is clear clones
  the template its header names (when it names one); one with the bit set is
  copied under the first free local id, keeping its string ids and adding a
  reference to each own string. The copy's table flags are read by the
  source's index after the insertion, so a copy inserted at or before that
  index takes another entry's flags (its own zero flags when it took the
  source's index).
- **Promotion**: a local object (tag exactly bit 31) comes back as its
  recorded world id when the id remap table holds it. Otherwise its container
  children are promoted first and their ids rewritten; each own string whose
  text it finds moves from the local library to the world library (one
  without text becomes inherit); it is inserted under the world tier's first
  free id with its table flags and deleted from the local tier; the pair is
  recorded in the id remap table and, for an operative object of kind 0, in
  the link remap table, each when switched on. A world object with a
  container flag promotes its children in place. A missing local object
  gives 0; untagged and both-tag ids come back unchanged. (The original
  recurses without a limit; Exarust stops 64 containers deep.)
- **Remap tables**: switching either table on or off empties it. The link
  rewrite (after an arena save loads) turns its table off, then, when it
  holds pairs, rewrites the linked-object word (door body `+4`) of every
  local record flagged as a door and empties the table. The entity rewrite
  maps a scene entity's object id that is tagged exactly local
  through the id remap table.
- **Removal** ("Remove DB reference"): only for local and world ids; an
  object with a random table section is kept; otherwise each model string id whose
  low 31 bits are not zero is released (with its top bit cleared) in the
  tier's library, and the record deleted. An unguarded deletion does the
  same for any tier without the random-table test (the base tier releasing in the
  shared library).
- **Template helpers**: an object's template is its parent word (or the id
  when it does not resolve); the bulk byte (model `0x1C`) is read from the
  template a tagged id's header names, and from an untagged id's own model.
  A weapon section being built takes the stat bytes `0x30`–`0x40` and `0x42`
  of the weapon section of the template its record's header names.
- **Item-use test**: by the operative section, 0 without one and for classes
  2, 4, 8 and 0 (class 0 kind `0x11` gives 2 when bits 16–23 of the caller's
  word are clear; for another kind the original first runs a use action:
  kind 7 hands the header name to a registered reader, kind 5 opens a form);
  1 for class 1 of kind 8, 9, 10 or 14 with those bits set; 2 otherwise.
- **Weapon variants**: a weapon section's word at `0x42` low five bits is
  the variant count, one less for the half-sword form (`flags & 0x30 ==
  0x10`), never below zero.

### Migration

An instance (tag exactly local or world, or every record of a tier after a
save loads) refreshes from the template its header names, when that resolves.
The material class compares bits 16–23 of the template's header word (read
at its payload `0x18`) with the instance's. Each instance section with a tag
of `0x1000` or more that the template also has is then refreshed:

- **Model**: with the class changed, the template's first 40 bytes with both
  string ids set to inherit; else the name, the flags word and the bytes
  `0x1C`–`0x1F` (bulk, rarity, value).
- **Body**: with the class changed, the appearance layers first (below).
  Then, when the template's paint seed (the word at `0x1AC`) is
  `0xFFFFFFFF` (take the template's):
  the instance's becomes so too, bytes `0x1B0`/`0x1B1` copy, each of the
  eight layers' byte `+7` (`0x1BB` + 12i) becomes `0x88` for a layer
  material (`+4`) of 1–25 and 0 otherwise, and each of the four power bytes
  (`0x218` + 2i) keeps its value with `0x80` set when the template's has
  `0x80`, else becomes 0. Else, when the instance's `0x1AC` word is
  `0xFFFFFFFF`, it takes the template's first `0x224` bytes. Then the bytes
  `0x00`–`0x0F`, `0x10`–`0x19F`, `0x1A6`–`0x1A8` and the word `0x220` copy,
  and the condition refresh runs (below). An instance seed of `0xFFFFFFFF`
  marks a copy not yet made from its template.
- **Weapon**: when the template's word at `+4` is `0xFFFFFFFF`, the
  instance's words `+4` and `+8` become so and `+0xC` copies; the flags take
  the template's low 28 bits; when the instance's words `+4` and `+8` are
  non-zero and byte `+0xF` is zero, the stat bytes `0x30`–`0x40` and `0x42`
  copy, and a factory weapon (model name starting with `@`) is regenerated
  by the item factory (not reproduced).
- **Container**: the word at `0`, the words at `8` and `0xA`.
- **Door**: the template's `0x200` flag is ORed in.

**Appearance refresh** (eight 12-byte layers from `0x1B4`): byte 0 and the
mask byte 1 copy; when the template's mask is not zero the set byte 2 copies
and the instance's material (byte 4) is checked against that set's materials,
taking the set's first when it is not among them; byte 3 copies; the
template's material replaces the instance's when it is not zero; unless the
instance's byte `0xB` is 5, it copies, and the colour (byte 8) with it when
the template's is 5. The procedural material sets are: 1 and 5 `1 2 3 4 7 8
9`, 2 `5 6 10`, 3 and 6 `1 2 3 4`, 4 `7 8 9`, 7 `13 20`, 8 `1 2 3 4 17 13
20`, 9 `5 6`, 10 `7`, 11 `11`, 12 `13 14`, 13 `13 14 15`, 14 `20`, 15 `23`;
other sets list none (the original then substitutes an uninitialised byte;
Exarust keeps the material).

**Condition refresh** (quality q and damage d at body `0x1B0`/`0x1B1`, all
from the template's bytes): k = (1 − d·0.4/255)·(q/510 + 0.8) (single
precision after a double product); protection bytes `0x1A0`–`0x1A3` =
round(t·k), capped at 160 (the low byte of the rounded value is stored
first); `0x1A4` = round((0.3k + 0.7)·t); `0x1A5` = round(t / ((1 −
d·0.15/255)(q·0.3/255 + 0.85))); then the armour variant (byte `0x1AA`,
1–8) runs its handler, scaling the four resistances and the encumbrance in
double precision, uncapped (`objdb::variant_scales`; shipped templates have 0
or 255 there, rolled copies a variant); and the model's value word (compact form) becomes v / (2tq² + 0.5) · (2q'² +
0.5)(2(1 − d')² + 0.5) · 0.5, with tq the template's quality and q', d' the
instance's, each /255. Rounding is to nearest, ties to even.

### The condition word of random-table rows

A random table's rows (section `0x200`, see
[Containers and random tables](#containers-and-random-tables)) carry a
condition word; this unit holds its getters and setters (`FractionWord`).
The setters OR the rounded value in without masking it, so an out-of-range
value spills into the next field:

| bits | field | getter | setter |
|---|---|---|---|
| 0–3 | wealth | value ×0.25 | round(v·4) |
| 6–8 | damage | value ×0.25 | round(v·4) << 6 |
| 9–10 | damage spread | value ×0.25, 0 reads 0.125 | below 0.1875 → 0; below 0.625 → round(v·4); else 3 |
| 11–13 | dirt | value ×0.25 | round(v·4) << 11 |
| 14–15 | dirt spread | value ×0.25, 0 reads 0.125 | below 0.2 (a double compare) → 0; else round(v·4) |

The old body converter writes `0x5284` (wealth 1.0, damage 0.5 and spread
0.25, dirt 0.5 and spread 0.25) into rows that lack one; the shipped tables
use `0x5284` 406 times, `0x5484` 15, `0x9484` 15 and `0x9486` 2 across 438
rows.

### Quantised item fields

The same unit holds the packed getters/setters the item descriptions, trade,
the arena and the randomisers share. They read byte 0 and byte 3 of a record
whose layout is not established:

| field | getter | setter |
|---|---|---|
| byte 0 bits 0–1 | 0..3 | clamps to 0..3 (NaN to 0) and rounds |
| byte 0 bits 2–6, sign bit 7 | n·(1/31) in double precision, negated by bit 7 | round(\|31v\|) << 2 (a value past 31 spills into bit 7), then bit 7 for a negative v |
| byte 0 bits 5–7 | the top three bits | — |
| byte 0 bits 0–4 | the low five bits | — |
| byte 3 fractions at bits 0, 3, 6 | n < 2 → n/8, else (n − 1)/4: 0, 0.125, 0.25, 0.5, 0.75, 1 | n = round(8v); above 1 → (n >> 1) + 1 capped at 5 (3 in the two-bit field at 6), ORed in unmasked (a negative n sets the bits above) |

**Amounts.** A `u8` + `u8` + `u16` triple of units 1, 12 and 144
(`b0 + 12·b1 + 144·w`); packing cuts the word to 16 bits. The add helper
adds 32-bit units, the subtract helper a second packed amount, the total
helper two packed amounts; a result of zero or less clears the amount. The
16-bit compact form's top two bits scale the low 14 by 1, 12, 144 or 1440,
the first scale that fits (below `0x4000`, below `0x2FFF5`, below `0x23FF70`,
else 1440). Exarust: `objdb::{amount_units, amount_bytes, amount_add,
amount_sub, amount_total, amount_compact, amount_expand}`.

### Corpus

- `objdb.rdb`: 1424 table slots, 1423 records and the slot after them;
  368,966 blob bytes (payloads 368,886, holes 80).
- Section counts: header 1423, model 1403, body 388, weapon 208, container
  137, operative 49, door 47, random table 24, shield 22, torch extra 1, map tag 1.
- Distinct table flag words: 11 (`0x10100000` 529, `0x10101000` 387,
  `0x10104000` 207, `0x10120000` 137, `0x10110000` 49, `0x10140000` 47,
  `0x10100200` 24, `0x10108000` 22, `0x10000000` 19, `0x10001000` 1,
  `0x10104100` 1).
- Random tables: 24, 438 rows, every table's first row the empty row (id 0).
- 22 levels carry a local database (`0xDBC0` → `0xDBDB` records, `0xDBCD`
  library): 3812 records, 1283 containers, 119 own strings.
- `actors.rpk`: 20 of the 21 actors carry a database, 17 of the `0D08`
  layout.
- Every one of these databases and libraries writes back byte for byte (the
  older actor databases with the current magic, as the original would save
  them); `objstrings.rdb`'s first entry is "Sword", entry 88 "Breastplate",
  89 "Plate Helm", 285 "Falchion", 170 "Torch", 182 "A simple torch."; every
  model name and description id of `objdb.rdb` that is neither 0 nor inherit
  resolves in the shared library.

### Evidence and precision

Established by reverse engineering of `Exanima.exe` 0.9.5: the lazy loader,
the database loader (count rule, upgrade table and dispatch) and writer, the
string library's loader, writer, getter, allocator, setter, add-reference and
release, the section finder, mask test and editing routines, the record view,
the id resolver, binary search, first-free scan, sorted insertion, payload
store and deletion, the string resolver, the template clone, instancer,
instance-template and template helpers, bulk helper, dispatcher,
duplicator, promoter (container recursion, string moves, remap tables), the
remap switches, the link and entity rewrites, both removal routines, the
item-use test, the migrator (by its assembly), the appearance refresh and
its material-set table, the condition refresh (by its assembly and
constants), the two old-section converters, the packed-field accessors and
the amount helpers (the float ones by their assembly). The conversions are
the processor's round-to-nearest-even (`cvtss2si`/`cvtsd2si`), whose NaN or
out-of-range result is the integer indefinite; the precision of each step
(single or double) follows the assembly; the armour variants' scales are
those the item-copy work established for the same handlers (`exa_items`,
now shared from `objdb::variant_scales`). Counts are from the installed
databases (`objdb::shipped`, `objstrings::tests`).

## Object database items and apparel painting

The section layouts, the three tiers and the string rules are above; this
section decodes the item fields themselves.

`objdb.rdb` holds 1423 records and the table slot after them (see
[Framing](#framing-the-table-and-the-writer)). An item record payload is a chain of `u32 tag, u32 size` sections,
where the size counts the 8-byte header. A record's table flags are the OR
of the tags of the sections it holds (1422 of 1423 records; the exception
is "Map"), so `0x10101000` is name, ground model and body, and `0x10001000`
name and body only.

| tag | contents |
|---|---|
| `0x10000000` | display name[16], a word (0, 1 or 2, else noise; [guess] editor state), the parent id: the editor folder for global items (`+Apparel/+Misc`, `+Clothes/+Legs`, …), the source item for level-local copies |
| `0x00100000` | ground or icon model name[16] (`Objects.rpk` or `Objlib.rpk`, or `@` and a factory), then a flags word: bit 0 dims the model's voxel lights and glow meshes by the placement's condition byte (salves `0x51`/`0x1`, power crystals `0x1031`, Diviner's Deck, Repair Sphere, PedestalHand; see `RFC.md`, placed objects), …; the byte at `0x24` from the section's start (body `0x1C`) is the equip lock the character record's conflict pass reads: over 50 it will not take the item off for another (121 items: chests, drawers and tables 200 or 255, the ogre's club, the golem suit and the ogre armour and greaves 100; [guess] a weight class); the item panels count the same byte as an entry's bulk, so a character list load sums it into the pack's used capacity; absent in "Braies_A" |
| `0x1000` | item body: the worn model name[16] (`Characters.rpk`), the occupancy table at `0x10`, the slot table at `0xA8`, coverage and protection at `0x150`/`0x1A0`, encumbrance at `0x1A5`, the appearance block at `0x1B0..0x214`, the special kind, power flags and articulated set at `0x214`/`0x218`/`0x220` ([Armour](#armour)) |
| `0x4000` | weapon section (208 items): flags, factory seeds and choices, stat bytes `0x30..0x42` (Weapon stats) |
| `0x8000` | shield section (22 items, 80 bytes): weight, block and bulk bytes at 4, 5, 6 (× 0.01 in double precision; Weapon record); the bulk adds to its holder's encumbrance ([Encumbrance](#encumbrance)) |
| `0x10000` | operative section (68 bytes; levers, buttons and plates, keys, signal jobs, relays, other usable objects): class, kind, flags, targets (Door and operative sections) |
| `0x40000` | door section (28 bytes): flags, linked object, sound letters (Door and operative sections) |
| `0x100` | held light (16 bytes; only "Torch A01"): the hands treat the item as a light ([Containers and random tables](#containers-and-random-tables)) |
| `0x200` | random table (24 records): a 16-byte header and 16-byte rows ([Containers and random tables](#containers-and-random-tables)) |
| `0x20000` | container (137 records): panel size, capacity, largest item, children ([Containers and random tables](#containers-and-random-tables)) |

**Worn and ground model.** A character is dressed in the model the body
names, never the `0x00100000` one: REA on the apparel model loader shows it
loading and caching the body's leading name from `Characters.rpk`, and the
icon builder gives no icon when the `0x00100000` section is missing. Across
`objdb.rdb` and every level's own database the two names agree for 1617
apparel records (363 global, 1254 local). They differ for item 8
"Braies_A", which has no ground model, and for "Legg Cloth Lay A", "Trouser
Qui A" and their 39 local copies, which are worn as `Trouser_A` and shown
on the ground as `Trouser_At` (an `Objects.rpk` entry). 366 of the 388
objdb records with a body are worn as a `Characters.rpk` model (319
models), against 363 by the ground model. Exarust reads both
(`Item.worn_model`, `Item.model`): apparel is dressed, painted and looked
up by its worn model, weapons by their ground model. `Item.rank_word` is the
trade rank word resolved by the body/weapon/shield order below, and
`Item.armour_type` the body's armour type byte at `0x1A8`. The arena's
item-fact pass reads those, the appearance bytes and the occupancy table
(`ItemFacts::of_item`).

Appearance block (100 bytes): quality, damage and dirt bytes (the painter uses
q = x, damage = x/(2−x), dirt = (x²+x)/2 with x = byte/255), their packed
spreads ([Armour](#armour)), then eight 12-byte layers: +0 flags (`0x1` the
first layer's material sets the armour variants a leather adds and later
such layers copy it, `0x2` layers of one random set share a material), +1
mask number (`<stem>m<n>`, 0 = unused), +2 material set (used when the
material is 0), +3 wealth level, +4 material, +8 palette colour, +0xB
colour mode (5 once rolled). The roll of a copy (`exa_items::palette`,
REA on the apparel roll) draws a missing material from its set: sets 1, 3,
4, 5 and 8 weigh each material by `reach − |level − its level|` (the
material table, levels 0–3) times the set's weight, set 12 takes the
plain plate below the palette's plate level and its plate above it, set 13
the palette's mail, the others pick evenly; the colour follows the
material's class and the layer's colour mode.
The original's texture job reads all eight layers (its loop runs while the
index is below 8; no shipped item fills the eighth), skips a layer with mask
0 and stops at the first layer whose mask the texture set lacks.

**Occupancy table** at body `0x10`: a `u64` for each of bones 1..20 (bone b
at `0x10 + 8(b − 1)`). Byte L is a bit mask of the bone's sub-parts the
item occupies on layer L, 0 innermost. Established by REA: the apparel
loader takes an item's layer as the highest non-zero byte over the table
and keeps worn items sorted by it; putting an item on takes off every worn
item whose table overlaps the new one's on any bone; the underwear rule
tests bit `0x02` of the pelvis's layer-0 byte, the groin. Only that bit is
tied to behaviour; the other sub-part bits are not decoded. Trousers, hose
and braies set it (pelvis `0x03` on layer 0); mail legs sit on layer 1,
shirts and gambesons on 2, tunics and coats on 4, plate on 6–7. The table's
last entry (bone 20) is the first word of the slot table at `0xA8`.
Exarust reads it (`Item.occupancy`, `Item::covers_groin`) for the
underwear rule; wear conflicts wait for equipping.

Item body offset `0xA8` holds apparel's per-slot layer table: 8 bytes for
each of 19 body slots (bone indices). Low bits give the layer level. Bit
`0x80` hides the body part of that slot (helm: head; trousers: thighs and
shins; boots: feet; gambeson: torso and arms). Bit `0x40` hides the slot's
partial body variant. Exarust hides covered body parts' meshes when
dressing (from the first byte of each slot, `Item::hidden_slots`).

The character builder ors the whole eight-byte words of every worn item
(`Item::slot_words`) into a union, then runs one pass per worn garment, in
layer-level order (the highest layer any of the garment's occupancy words
holds). For each slot it walks the garment's occupancy bytes from the
outermost layer in: at layer L the union's bytes above L join the slot's
cover where the garment holds nothing higher, a union byte with `0x80` hides
the garment's slot unless its higher bytes meet the slot's mask, and one
with `0x40` hides the slot's partial bit under a second mask; a garment
holding none of a mask at all hides it too. A garment node's name gives its
slot and a sub-part bit set as four digits (`0906Chest`: chest, bits
`0b110`); the assembler scores each node's bits against the slot's cover
(matched low bits plus sixteen, unmatched bits costing eight each), keeps
the best groups slot by slot (child slots are searched under the group's
parent-slot node, else under any parent-slot node of the same bits), and
merges the first group's node of each slot whose select mask (the slot's
bit, plus partial and shoulder bits for some) meets the garment's visible
mask. Once every garment is merged, the body model's parts are added where
the union's innermost byte lacks `0x80` (and `0x40` for the partial bits).
Exarust's `exa_character::layers` reproduces these passes as tested rules;
`equip_outfits` still hides whole body-part meshes by the first byte. The
sub-part bits themselves are not decoded. Fit variants and culling of
apparel triangles by their cull bits are not done yet.

### Containers and random tables

Read by `exa_formats::containers`. Established by REA on the container
panel's loader and saver, the world-load fill and the item-instance
helpers, and checked against `objdb.rdb` (offsets are section-body
offsets, after the 8-byte header).

**Model section** (`0x00100000`, 40-byte body): the flags word at `0x10`,
the item's bulk byte at `0x1C` (what a container's capacity counts and what
a slot's largest item limits), and the compact price word at `0x1E`. A
price word's low 14 bits count single copper with neither top bit set,
twelves with `0x4000`, gross (144) with `0x8000` and 1440 with both;
storing an amount takes copper below `0x4000`, twelves below `0x2FFF5`,
gross below `0x23FF70` and 1440 above, dropping the remainder.

**Container** (`0x20000`): u16 width and height in GUI units at 0 and 2,
u16 capacity at 8, u16 largest item at `0xA`, u32 child count at `0x14`,
then 12-byte children `{u32 id, u32 flags, i16 x, i16 y}` at `0x18`, the
position in 1/80 of a GUI unit. The shipped chests are 65 × 50 units with
capacity 200 and largest item 50, and hold no children in `objdb.rdb`; a
level's own database lists what its containers hold. A child whose id
has a random table is filled from it when the world loads. An empty child
is stored as id 0 with flags `0x100`.

**Random table** (`0x200`): a 16-byte header with the count draws' minimum
and maximum bytes at 4 and 5 and the u32 row count at `0xC`, then 16-byte
rows: u32 id at 0 (0 is the empty row, always first in the shipped
tables), weight byte at 4, decay at 5, quota at 6, limit at 7, u16 flags
at 8 (`0x2` the row is only for the count draws; `0x8` is set at load when
the row's id has a table of its own, which a pick then descends into), the
condition word at `0xA` and the bulk byte at `0xF` (copied from the item's
model section at load). The condition word holds wealth in bits 0–3 (×
0.25), damage in 6–8 (× 0.25) and its spread in 9–10, dirt in 11–13 and its
spread in 14–15 (a spread code of 0 is 0.125, otherwise × 0.25). Shipped
tables (24) have header bytes 0, no nesting, no quotas, one row with a
limit of 1, and flags 0 or `0x80` (`0x80` not decoded); "Clothes Tier 01"
rows carry condition words `0x5284` and `0x5484`.

The fill (`exa_items::randomise`) builds one table per id in the order
the containers name them, then for every table: quotas to random empty
slots that fit, count draws among the count-only rows, then the weights
(a row weighs `w·1000` unpicked, `(1 − decay·0.005)·w·1000` after one
pick, the engine's fast `exp(n·ln(1 − decay·0.005))·w·1000` after `n`,
and 0 at its limit; a nested row weighs its table's total less its empty
row's, times `w/100`), and a weighted draw for every slot still empty that
passes over count-only rows and rows bigger than the slot allows. Every
slot draw uses the engine LCG. What instantiating a pick does to the item
(a fresh local copy, the row's damage and dirt with their spreads, the
apparel palette roll) is described in `exa_items::instance`; the copy
itself is not made yet.

### Underwear

The human body is nude: `humanbase.rfc` has no underwear part and the skin
textures are painted bare. Underwear is an item, objdb record 8
"Braies_A", and the engine refers to it only by that id.

- **The item.** Worn as `braies_a` (one node on the pelvis, texture set
  `braies__aa`, which has masks 1 and 2 only). No ground model, so it has
  no icon and cannot be dragged off the figure ([inf] from the icon code;
  not tried in the game). Occupancy: pelvis `0b11` and both thighs bit 0,
  on layer 0. Its slot table is all zero: it hides no body part.
  Coverage: pelvis 60 %, thighs 100 %, with zero padding and resistances
  and the cloth hit sound. Appearance: quality 160, damage 64, dirt 0;
  masks 1 and 2 rough cloth (material 1) in colour 1, palette family 0
  shade 1, `#c7c7c7`: pale grey linen. "Braies_P" (425) is the same model
  shown on the ground as `braies_a` and painted in mauve silk (colour 250);
  no level, local item, container, drop table or actor uses it.
- **The rule.** If a character record's race byte is 0 (human; [inf] that
  the tested byte is the race byte) and none of its worn items covers the
  groin, item 8 is appended to its worn list. It runs when a record is
  created, when the character screen opens, after an item is taken off the
  figure, looted or stripped, and in the arena's outfit generators. It does
  not run at spawn: a placed character is dressed exactly as stored.
- **Displaced braies vanish.** Putting on anything that overlaps them
  (trousers, hose) takes them off, and every handler of displaced items
  drops item 8 instead of storing it, so braies are never carried. Taking
  the trousers off again brings them back through the rule.
- **Start outfits.** A fresh record wears `[8]`, also in the play-start
  fallback with no player file. Story background 0 ("unknown") is cut to
  braies only, plus a start letter in the pack; its creator kit offers a
  shirt, trousers and shoes. The proctor and villager backgrounds use their
  `actors.rpk` templates, which wear trousers; 0.9.5 has no template for the
  knight. No kit, template or level lists braies for the player.
- **Corpus.** Of the 103 placed humans, 4 wear item 8 and nothing else
  (Sticky and two dead men in exanima01, Fergus in exanima02) and 99 wear
  trousers: all 103 are covered by their stored items.
- **Evidence.** REA on `Exanima.exe` 0.9.5 (the rule and its callers, the
  record defaults, the creator's background presets, kit tables and mode
  filter, the equip conflict pass and the displaced-item handlers, the
  apparel loader), the corpus above, and a retail save whose fresh story
  player (background 0) wears item 8 only, as do its Sticky and two
  unnamed humans.

Exarust (`exa_character`) applies the rule to the humans it makes at run
time: the player in play mode and the character view's human body. The
underwear goes on beneath the given outfit, painted with item 8's own
appearance, when nothing in the outfit covers the groin. Placed characters
are dressed as stored. Equipping (`exa_items::carry`) takes off every
worn item whose occupancy overlaps the new one: braies are dropped, other
items return to the pack, and a human left bare at the groin wears braies
again. The inventory windows are not in.

A feral character's dressing and re-texture passes repaint the underwear
it wears: its appearance's damage and dirt bytes (the block's second and
third) become `(2 − f)·f·255` (the feral
share, converted with round-to-even and cut to a byte) and the compositor
seeds it with the record's look seed rather than the item's own. Every
other apparel item keeps its stored appearance and seed
(`exa_character::rebuild::retexture_layer`).

The game never draws an apparel item's own textures. It paints a colour
texture per item from the shade map `<stem>d` (cavities dark), stacking each
layer's finish over its mask on black. Run-time materials `&<10 chars><digit>`
name the stem: the ten characters after the `&` (`&doublet_aa1` →
`doublet_aa`, `&gambsonaba1` → `gambsonaba`). A model gets one job and one
stem: the walk takes the first `&` material in node and part order and
rewrites every `&` part to that job's stem (`exa_assets::model::
painted_model_stem`). The finishes by material number:

| id | material | finish family | colour-pass textures (sampler order) |
|---|---|---|---|
| 1 | ClothBasic | cloth, painted pattern | `ClothRghA03d`, `LeatherDmgA02d`, `ClothDmgA01d`, `DrtApr01` |
| 2 | ClothFine | cloth | `ClothFinA01d`, `LeatherDmgA02d`, `ClothDmgA01d`, `DrtApr01` |
| 3 | ClothSilk | cloth, silk specular | as ClothFine |
| 4 | ClothVelvet | velvet | `ClothFinA01d`, `LeatherDmgA02d`, `ClothDmgA01d`, `DrtApr01` |
| 5–6 | ClothPadded1/2 | cloth, clothp normals | `ClthP01d`/`ClthP02d`, `LeatherDmgA02d`, `ClothDmgA01d`, `DrtApr01` |
| 7–8 | LeatherA02/A01 | leather, painted pattern | `LeatherA02d`/`LeatherA01d`, `LeatherDmgA02d`, `DrtApr01` |
| 9–10 | SuedeA01 | suede | `SuedeA01d`, `LeatherDmgA02d`, `DrtApr01` |
| 11 | SteelChain | chain | `MailStlA01d`, `MtlDirtA01d`, `MtlRustA02d`, `MtlScufA01`, `MtlRustA01g` |
| 12 | SteelGen | steel, alt normal | `MtlStlBas01d`, `MtlImpure01d`, `MtlRustA03d`, `MtlScufA01`, `MtlRustA01g` |
| 13, 15 | SteelPlate2 | plate | `MtlStlBas01d`, `MtlImpure01d`, `MtlRustA03d`, `MtlScufA01`, `MtlRustA04g` |
| 14 | SteelPainted | painted steel | as SteelGen plus `MtlDirtA01d` |
| 16 | SteelBlade | blade | `MtlStlBld01d`, `MtlRough01d`, `MtlRustA02d`, `MtlScufA01`, `MtlRustA01g` |
| 17–18 | WoodFineA/WoodRoughA | wood | `WoodFineA01d`/`WodHft01d`, `MtlScufA01`, `MtlDirtA01d` |
| 19 | IronGen | steel, alt normal | as SteelGen |
| 20 | CopperAlloy | copper alloy | `MtlStlBas01d`, `MtlImpure01d`, `MtlRustC01d`, `MtlScufA01`, `MtlRustA01g`, `Verdigris01d` |
| 21 | CopperSatin | copper satin, copper-alloy normals | as CopperAlloy |
| 22 | WiredGripA01 | leather | `GripLtrA02d`, `LeatherDmgA02d`, `DrtApr01` |
| 23 | Gemstone | gemstone, alpha, pattern | `MtlStlBas01d`, `MtlImpure01d`, `MtlRustC01d`, `MtlScufA01`, `MtlRustA01g`, `DirtBrownA01` |
| 24 | Resin | resin, alpha | as Gemstone |
| 25 | PlanksPainted | painted planks | `WodPlnkP01d`, `MtlScufA01`, `MtlDirtA01d`, `PaintMskA01` |

Material 0 takes the first material of its set (sets 1/3/5/6/8 → 1, 2/9 → 5,
4/10 → 7, 7/12 → 13, 11 → 11, 13 → 13, 14 → 20, 15 → 23). An unknown id
reports "Invalid Procedural Material ID." and paints as ClothBasic. The
specular and normal passes have their own lists (for example metals read the
impurity map for quality, the rust normal for damage, a colour map's red as
the rust mask and a full-resolution scuff normal); the registry is in
`exa_formats::procedural`.

Each pass samples its textures through the program's tile uniforms:
material-scale textures (weave, worn weave, impurities, quality) repeat
once per as many target texels as the material's first colour texture is
wide (`MatTile`, the same divisor in all three passes), damage, dirt, rust
and scuff maps once per 1024 target texels (`DmgTile`), and the steel
normal programs lay their scuff normal once over the region. The blade
reads its quality maps at twice the material scale. The cloth programs'
specular pass reads its damage and dirt at the material scale, the hide
programs' at the damage scale. Each layer's `MatCol` is its colour and a
wear weight: on apparel the material's (1; velvet 0.8, SteelGen 0.75,
materials 21–25 0.5), on a factory region the area's finish level. The
colour and normal passes run over every layer before the specular pass,
whose crease measure reads the painted normal map ±2 texels around.

Palette: index = family × 8 + shade. Each of the 32 families has hue,
saturation, base value and value step. With k = 7 − shade, the linear ramp
(apparel) is v = min(k·(step·0.13) + base, 1) and the lifted ramp (factory
material rolls) is v = min((k·(step·0.13) + (base + 0.1))², 1), the
parenthesised sums and the square in double; both use s = (1 − (0.09k)²)·sat
in double, scaled by (v + 0.6)² (double) when v < 0.4, then HSV → RGB with
the sector ⌊6h⌋. Family 0 is the greys (0.91 down to black).

**Colour space.** REA on the original's paint jobs and texture loading shows
the following.
- The finishes read the shade map, the masks, the item normal and every
  library texture as stored. All are loaded as data and none is decoded from
  sRGB.
- The palette colour reaches the programs as computed (HSV to RGB, not
  linearised).
- The colour target is sRGB RGB8 (sRGB RGBA8 for items with alpha). The job
  switches sRGB encoding off before drawing, so each texel stores the
  program's output unchanged.
- The scene samples the colour target through sRGB decoding and multiplies it
  by the painted material's diffuse colour, which is 1.
- The specular target is linear RGB8 and the normal target linear RG8.
- The GPU builds the mips of all three targets after drawing (the original
  skips them when the GL driver reports a Radeon).

A finish's output is therefore an sRGB value, not a linear albedo. Pale grey
rough linen (palette colour 1, the braies' quality and damage) over a mid-grey
shade map and mid-grey library textures paints texel 96 and is lit as 0.117.
Exarust keeps this chain. It uploads the colour map as sRGB with the texels
as painted and averages its mip levels in linear light ([inf] that the
original's driver builds sRGB mips in linear light, as it filters them). The
surface and normal maps are uploaded as linear data.

Exarust paints on the CPU (`exa_assets::paint`; `exa-tool paint <model>
<stem> out.png`). It transcribes all fifteen colour programs and their
surface, specular and normal passes: cloth, silk, velvet, leather, suede,
wood, steel, blade, plate, painted steel, chain, copper alloy, copper satin,
gemstone, resin and painted planks. The surface target keeps the original's
packed layout (intensity in red, gloss in green, metal in blue); the scene
reads roughness = 1 − gloss, so Exarust no longer copies gloss and drops
intensity. A factory item's paint job takes quality and dirt as the bytes
over 255 and the damage as 2/(2 − x) − 1; apparel takes damage x/(2 − x)
too and dirt (x² + x)/2. The rest of the factory job is under Weapon
factories (Paint).

Not yet painted: the per-area emblem layers (the factory appends an extra
layer in another palette colour with a generated emblem texture; the emblem
generator is outside the factory), pattern opacity textures, the factory's
shared atlas (each part is painted on its own at its component's
resolution) and per-instance randomized appearances. The painter samples
the nearest texel where the GPU filters.

### Trade fields

What a shop and the story's item discovery read of a record
(`exa_formats::trade_record`), offsets in section bodies, the first
section of each tag counting. Established by REA on the shop's stock
update, its rank helper and the story's pickup hook; a field past its
section's end reads as zero.

| section | offset | field |
|---|---|---|
| `0x10000000` | `0x10` | listing word: bit `0x2` keeps the item out of every stock |
| `0x10000000` | `0x14` | parent id: the source item of a level-local copy (a shop shuns items whose copies it holds) |
| `0x00100000` | `0x00` | model name: a stocked item needs a non-empty one |
| `0x00100000` | `0x10` | flags: bits `0x70` are the rarity tier 0..7, which prices a new kind's discovery at 10, 20, 50, 100, 200, 500, 1000 or 2000 points |
| `0x00100000` | `0x1D` | rarity in hundredths: a shop weighs the item by `s = 1 − rarity/100` |
| `0x00100000` | `0x20`, `0x24` | two words a stocked item needs non-zero [guess: the icon's size] |
| `0x1000` | `0x1AB` | rank in the top three bits (apparel) |
| `0x4000` | `0x42` | rank in the top three bits (a weapon without a body) |
| `0x8000` | `0x08` | rank in the top three bits (a shield with neither) |

The rank is read from the body, else the weapon section, else the shield
section, else 0. A shop's new apparel copy rewrites its body's condition
byte at `0x1A9` (low two bits kept, `round(|31q|)` in bits 2..6, bit 7
for a negative quality `q`), and the appearance bytes at `0x1B1` (damage,
0x40), `0x1B2` (dirt, 0x35) and `0x1B3` (5 in bits 3..5, 3 in bits 6..7).

## Door and operative sections

Status: **verified** for the fields below (REA on the original's door,
lever and push-surface set-up, its signal dispatch, relays and jobs, and the
object factory that picks the body class; counts from the installed
databases). Offsets are from the section body (after the 8-byte header).
Exarust reads both into `objdb::Item` (`door`, `operative`).

**Door section `0x40000`** (20-byte body):

| offset | field |
|---|---|
| 0 | u32 flags: `0x2` latches (bars itself shut when it closes), `0x4` locked, `0x8` not a door to the hand's click, `0x10` motorised (moves by itself when signalled), `0x40` fast motor, `0x80` signals inverted, `0x100` a sliding door held at its file position, `0x200` sets scene-node flag `0x80` (meaning not established) |
| 4 | u32 linked object (database tag bits as placed objects): the key that unlocks the door, compared with the held key; relays of kind 1 signal every door linked to them; some doors name the lever that moves them |
| 8..11 | sound letters: lock (`LkO`, `LkT`), slam (`slm`), hinge creak loop (`hng`), unlatch (`opn`); the sound is `door`, `a` + letter, `_` and the suffix (`doorf_slm` → `doorf_slm01…06` in `Sound.rpk`) |
| 12..19 | zero in every record |

Flags seen across the databases: 0 (plain), `0x2` (secret door), `0x12`
(city gate), `0x52`, `0x108`, `0x200` and `0x204` (prison gates,
portcullises), `0x214`, `0x216`, `0x254`, `0x256`, `0x2d4`, `0x5c` and `0xdc`
(pit doors: locked, not clickable, motorised and fast; `0xdc` also
inverted). In `exanima01` the linked objects are 6 keys, 4 levers and 7
kind-1 relays; one door names an id its database lacks.

**Operative section `0x10000`** (60-byte body):

| offset | field |
|---|---|
| 0 | u8 class: 1 key, 2 device (a teleporter or reconstruction device: kind 15 or 18), 3 lever, 5 push surface (buttons, pressure plates), 6 job, 7 relay; 0, 4 and 8 are other usable objects not decoded here |
| 1 | u8 kind: a lever's 6 fades a light under its node instead of signalling; a push surface's 2 inverts its signals; a job's `0xC` lights a fire, `0xD` turns a light on or off; a relay's 0 and 3 combine their inputs' bits into a code, 1 signals every object linked to it, 2 signals both targets, 5 counts a sequence |
| 2 | u16 flags (a lever's starting value and forced detent, a push surface's one-shot and second-target bits, a relay's inversion and quiet bits) |
| 4 | u16 value: a fire's size, a relay's state, the bit an input sets in a relay |
| 6 | u16 a lever's auto-return time in 100 ms (0: it stays) |
| 8 | u32 the object signalled |
| 12 | u32 the second target, or a relay's code |
| 16.. | a name (`StnSwtch01`, `DeviceCmbA01`) and further words in some records of the other classes, a script line in one; not decoded except for a teleporter |
| 16 | a teleporter's 16-byte name, zero padded (`PrtlSmblA.rfi`); the use loads the screen's atlas `prtlsmbla.rfi` by its own name |
| 32 | a teleporter's seven u32 destination level ids (locale list ids, 0 empty); its screen walks the first four, or only the first when flag bit 2 is set |

**Body class.** When the original loads an object's model, a record with an
operative section makes the model's root rigid body a lever (class 3) or a
push surface (class 5), and none of the others; a record with a door section
and no operative section makes it a door. No shipped record has both
(`Item::mechanism`).

Teleporter fields: the teleporter use hands the screen the body's word at
byte 32 as its destination list and tests flag bit 2 for the single-row
layout (read in the device unit's assembly); the shipped `TeleporterUBB`
(`0x555`, class 2 kind 15, flags `0x40`) names `PrtlSmblA.rfi` and the ids
7, 10, 12 and 13 (Exanima05, 07, 08 and 67, the levels that place one),
and `ResDevice` (`0x556`) is class 2 kind 18 (read from `objdb.rdb`).

Corpus: `objdb.rdb` 47 door and 49 operative sections (15 keys of kind 0;
47 doors, 6 levers and 5 push surfaces as bodies). `exanima01`: 43 doors and
72 operative records: 40 levers (39 of kind 1, one of kind 12), 12 push
surfaces (11 of kind 1, one inverted), 6 fire jobs (sizes 200, 176 and one 0), 8
relays (kinds 0, 1, 3, 5) and 6 keys. The other story levels have 2 to 8
doors each. `exa-tool mechanisms [level]` prints them.

## Saves (`.rsg`, `.rcp`)

Moved to [SAVES.md](SAVES.md) (package verified, slot rules verified,
player-data framing verified, chunk payloads partly known). In short: not
shipped; created in `%APPDATA%\Exanima` as `ExanimaNNN.rsg` or
`ArenaNNN.rsg` (with `.rcp` checkpoints), magic `0xDA7AEA02`, a 24-byte
section directory padded to `0x2000`, `#playerdata`, `#worldchrdb`,
`#worldobjdb` and a full modified copy of every visited level's `.rfc`.
Character records (`0x00CDACvv`, the same as actors and the levels' own;
layout in [RFC.md](RFC.md#starts-characters-and-placed-objects)) hold body,
race, equipment, health, personality, skills, roles and relations. Exarust
writes the same container and player-data framing for its own session.

**Settings chunk** (`exa_formats::save_chunk`): the header `0xECADCA01`, a
twelve-byte tag and two dwords; the old flag is set when the tag's version
float is below one million, and the easing curve is two quadratics joined
at 0.5. Evidence: the three retail routines behind it (the save-settings
writer, its reader and the curve).

**Character creation** (`exa_game::create`): the creator edits the
record's gear: its four hand items (id and flags), the worn list and the
carried list. A new record takes all four hand items, the worn list and
the carried list; the source keeps the hand items and loses both lists.
The mode filter keeps story background 0 in braies only and clears its
hands; the arena keeps the first two hands only when its preset kit offers
them and keeps braies only; modes whose 2 bit is set clear the fist
placeholder (31) from the first two hands. Each of those three filters
empties the carried list; a named story background and the other modes
are left as they are. After the filter every record but a recruited or
continued one gets the underwear pass: a record whose profession word has
no race band (bits 8 through 15) and whose worn items' apparel records
none carries flag `0x2` has the braies (item 8, no flags) appended (the
same rule the arena's loadouts use). The
skills are six slots of a `u16` points word and
six technique bytes; the free creator writes Close Combat, Armour and
Shields to 25000 points and their first five techniques, leaving the other
slots and each sixth technique. Each story background keeps its preset in
the settings under the path `0x6431bc02 + background`, `0x6431cc01`, and
the arena keeps its career character under `0x6431bc01`, `0x6431cc01`. A
loaded preset loses its race byte, known powers and power slots; the
unknown background then gets two known powers (`0x101`, `0x201`), its
first two slots (`0x100101`, `0x201`), the power word `0x510000` and 4.0
beside the slots, and a record with known powers opens the power tree when
it is committed. Entering the creator writes four `0xffff` halves into the
record: yellow, red, focus and the save seed. Evidence: REA on the
character screen's unit routines and the record layout above.

**Arena chunks** (`exa_formats::arena_save`): status **specified from the
retail writers and loaders, checked on synthetic bytes** (no shipped file
holds them). Framed `u32 id, u32 length, payload`; a string is a `u32` byte
count and its bytes.

| chunk | payload |
|---|---|
| company `0xA0DC0000` | name, a second string (empty in a new career), the 28-byte emblem record, three words, one unused word (skipped on reading), sixteen record words (the thirteen of the records window, three unused) |
| day `0xA0D10000` | today's matches, copper spent, copper earned; a count of special matches and per match its template id, rank, flags and an unused word; a word beside the log; the log's count and its twenty-byte records |
| tournament `0xA0600000` | the week's prize word, the six event rows (0x120 bytes), the sixteen rival companies (sixteen words; a first word of 0 makes the loader draw a new field), per event its template id, rank and two more words, then four week words and the eight-word prize history |

On loading, a special match is copied from the special table by its id (an
id the table lacks takes its last row) and rolled again for its slot; a
tournament event takes the tournament table's row of its id. An old save's
item lists are compacted: entries whose first word has the top bit
(`0x80000000`) set are dropped, and a special entry (either top bit of
`0xC0000000` set) removes the later twelve-byte records that repeat its
first word (`retain_plain_items`, `retain_plain_records`,
`remove_duplicate_item_ids`).

## Terrain (`.rft` and `.ftb`)

Moved to [TERRAIN.md](TERRAIN.md) (verified).

## Powers (`.pwr`)

Status: **the node layout, the text chunk, the link curve, the node
pictures, the learned marks and the thick joint are verified**. A blast
charge, its growth and the chunk separation are in `exa_game::thaum`.
A barrier ripple and the dome extent are in `exa_game::thaum`. A wave
charge, its charging radius and the released front are in `exa_game::thaum`.
A shell ripple is in `exa_game::thaum`. A torrent mix, its stream mesh counts and its push are in
`exa_game::thaum`. A shell vertex, its hit strength and its copied index list are in
`exa_game::thaum`. The compacted vertex map, the section slot and the index
room are in `exa_game::thaum`. The stored size of 800 and the fade that
divides by it are in `exa_game::thaum`. The light that follows the fade is
in `exa_game::thaum`. The second light's two values and the index upload
sizes are in `exa_game::thaum`. A play draw submits that upload on
the mesh when the flag is set. The Bevy render graph does not issue
the original's GL calls. Exarust:
`exa_formats::power_tree`.

The Mind power's spirit form (`exa_game::mind`) follows `TPwrSpiritForm`'s
tick.

- **Header**: first word `0xAFCE01CE`. The node count is the signed word at
  byte 52. A negative count is an error. The bytes between the magic and
  the count are skipped.
- **Nodes**: power id, the index of its name in the text table, the index
  of its description, x and y as floats, then a signed link count. A count
  below 1 stores no links. Otherwise that many 16-byte links follow. After
  every node is read, each node but the first subtracts the first node's x
  and y.
- **Text chunk**: first word `0xABCD0000`, then five words, then a 16-byte
  head whose last word is the *last* string's index, so the chunk holds one
  more string than that count. One more offset than that count follows,
  then a byte length and that many bytes. Each string is a length word at
  its offset and that many bytes. A length below 1 is empty. An offset
  outside the text leaves that string empty: the retail Force tree's table
  holds 19 offsets that point past its text. No chunk leaves the texts
  empty. A chunk that ends early is an error.
- **On screen** (`exa_game::thaum::tree`): a node sits at ten times its
  place times the zoom, plus the pan, and rounds to the nearest even integer
  unless the view is dragged. A factor then scales that point about the
  origin. A link whose two marks are
  zero is not drawn. Otherwise each end leaves its node along the sine and
  cosine of its turn (the link's first two words, fractions of a circle)
  times a full circle. The reach is 0.514, or 0.295 when the power's high
  mark is set, minus 0.003 times the zoom, times the zoom rounded to an even
  256. The samples are the cubic between those ends. A zero source mark dims
  the line. The fast sine polynomial and the thick-line quads are not
  reproduced. A turn of zero is straight up.
- **Pictures**: an icon with no columns is not drawn. Otherwise the cell is
  the power's low byte, one less, across the sheet, and the square is
  centred on the node. A second byte of 1 uses a bias of 1; any other uses
  the other bias. A frame uses the slot one less on a four-by-two sheet, at
  four fifths of the size. A wide picture drops the fraction of the view; a
  narrow one uses half. A glow is 32 spokes on a ring of 0.84 of the width
  plus the outer size, starting straight up.
- **Learning** (`exa_game::thaum::tree`): a node's mark is the last known
  pair whose id, with bits 20 to 23 cleared, equals the power. The first node sets bit 0x10000. A mark
  that already has bit 0x1000 keeps bit 0x4000 and sets bit 0x10000. Any
  other mark clears bit 0x4000, and sets bit 0x10000 when the top nibble is
  not 7 and every link's one-based end has bit 0x1000. A missing end does
  not.
- **Thick joint**: two points sit around the middle, one to each side of the
  chord from the first point to the third, at the given width. A width of 2
  through 0, 10 and 20 sits at 10, −2 and 10, 2 with a join of zero. A
  zero-length chord stays on the point.
- **Blast charge**: the rate is the tick in milliseconds times 0.001, then
  five sixths of that. Charging adds the rate. Passing 1 clears the amount,
  sets the state to 1 and releases. An amount of exactly 1 keeps charging.
  Fizzling subtracts twice the rate, and passing below 0 becomes 0 with
  state −2. After release the amount rises by three times the rate. A chunk
  grows by the tick times 0.7 of its speed plus 3, times 0.0006. Two chunks
  closer than twice the first radius move apart by half the overlap.
- **Barrier ripple**: strength falls by 0.05 a step. The shown value moves
  toward that strength by at most 0.04. Both under 0.07 removes the ripple.
  While it remains, its place eases 0.05 of the way toward the hit. The dome
  extent is 30 plus 1.25 times the ripple count.
- **Wave**: the rate is the tick in seconds, then five thirds of that.
  While charging or fizzling the radius grows by 0.0015. Once released the
  reach grows by 0.6, the radius by 0.0006 of the speed, and the trail step
  is 0.07 of the speed plus 1. An amount of 0.99 releases on a 12 ms wave
  rate. The blast rate for that tick leaves the amount at 1.
- **Shell ripple**: strength falls by 0.04 times the scale. The shown value
  moves toward that strength by at most 0.06 times the scale upward and 0.04
  times the scale downward. Both under 0.05 removes the ripple. The stored
  scale is 1. A vertex sits at its position plus 2 times the normalized
  bone sum, and a zero sum adds nothing. A hit of 0.1 stores about 3.333.
  A hit of 0.3 stores 6. An eighth hit is refused. A visible section
  copies its triangles from the first through the last. Each triangle keeps
  its three indices in order, after the remap. A hidden section is skipped.
  A range that ends early copies nothing. A short list or a missing remap
  is an error. A skinned section is copied only when bit 2 is set. A
  hidden section that also has bit 2 is still copied. A section without bit
  2 is skipped. A mark list starts at 255. A skinned section writes its
  index, and any other section writes 255. A vertex takes the next id at
  the first mark other than 255 in its half-open link range, and a vertex
  with none is absent. An index becomes that id. A kept vertex stores
  its section and the corner index minus that section's base. The first
  such mark wins, and a later link is ignored. A base of -1 subtracted
  from 1 stores 2. A short corner list or a missing base is an error.
  The index room adds one triangle for each skinned span, including a
  hidden one. A plain span adds nothing. It asks for 12 bytes plus 8,
  plus 4 bytes a triangle. The kind is 3, the base is 0, the end is one
  less than the count, and the vertex end is one less than the kept
  count. A range that ends one early adds nothing. A range that ends
  two early subtracts one. The stored size is 800. While opening, the
  shown value is the elapsed time divided by that size times 1.5. Above 1
  it becomes 1 and leaves opening, or enters the hold state when held.
  The hold state moves on, and its stamp subtracts the size times the
  remainder, rounded to the nearest even integer. While closing, the shown
  value is 1 minus the elapsed time divided by the size. A value that is
  not above 0 becomes 0 and finishes. A held close stores 3 times that
  value and then shows 1. Any other state stays as it is. The light,
  while opening, becomes one half once the shown value's double is above
  0.666, and otherwise 0.75 minus the absolute gap from 0.75 to the shown
  value times 1.5. Any other state takes half the shown value. The light
  is one tenth of that. While closing, the gain becomes the shown value
  times 2.8. Any other state keeps its gain. The second light stores
  0.1 and 0.75 at rest. While opening, a shown value under 0.25 stores
  one tenth of its times-4 product with 2 minus that product, and 1
  minus that product times 0.25. A shown value of 0.25 or more stores
  0.1 and 0.75. While closing, the first value is one fifth of 2 minus
  the absolute gap from 1.5 to the shown value times 3.5, and the second
  is the shown value times 0.75. A hold keeps the pair. An index upload
  asks for 20 bytes plus 8 a channel, and a payload of the triangle count
  times the width. The target is 0x8893, the usage 0x88E4, and the access
  0x88B9. A finish unmaps only when the buffer name is above 0. A
  play draw of those spans, with the flag set, puts one index per
  triangle of the payload on the mesh and unmaps a name above 0. A
  clear flag keeps the counted room and does not unmap.
- **Torrent**: below phase 1 the level is (2 minus the charge) times the
  charge times 0.4. Otherwise the level falls by 0.01, and below 0.02 it
  stops. Phase 1 eases the two channels toward 0.5 and 0.6 by 0.06. Phase 2
  multiplies both channels by 0.96. A span rounds to eight tenths. A
  stream sits at 16 times its direction, then its height times 1.5 minus 30.
  The mesh asks for 13 groups for each stream plus 8, 12 bytes a group and
  8 indices a group. A reach adds 10. A hit raises the channels by 0.005 and
  0.01 of the age over the sample count, until the second is no longer under
  1.8. A point inside the reach moves by 0.01 of one minus its distance
  squared over the reach squared.
- Four domains: displacement, energy, force, mind. The Bevy render
  graph does not issue the original's GL calls.

## Weapon factories (`.fty`)

Status: **verified**. All 106 parse (v0 46, v1 34, v2 26). The generator
reproduces the original's choices; the reference generator matched all 143
factory weapons of a real save, part for part and material for material.
`exa_formats::factory`, `exa-tool corpus factories`.

- **Header**: `u32 magic` `0xAFCE0F0v`, 8 reserved bytes, name[16] (empty),
  atlas width and height, `i32` grips (0) × 8 bytes, `i32` material areas
  (at most 8 read) × 56 bytes: name[16], 8 candidates
  {material, bits, fixed colour, mode}, count at +0x30.
- **Parts**: `i32 n`, then per part `u32 flags` (bit 0 texture-only overlay,
  `0x10` random twist, bits 1–2 paint-region flags), name[16], position,
  rotation (radians), scale (cm), atlas rectangle (offset, then scale, in
  atlas fractions: the part's region of the factory's texture), a
  texture parameter, 3 deforms (16 bytes in v0, 28 in v1/v2: header
  type|axis<<8|flags, amount, pivot, range, blend, stat weight, 0), {chance,
  attach-to (1-based), role, attach mode}, then `i32` variants of 40 bytes
  (44 in v2): model[16], mask areas[4], slash/crush/pierce/thrust bonuses,
  weight (0 → 100 before v2), tags, exclude mask, flags, and in v2 a texture
  scale, a child Y offset (mm) and two texture offsets. A trailer lists the
  tag names. v0 deforms carry no blend or stat weight; the parser fills in
  0.3 and 1.
- **Generation** (32-bit LCG `s·0x08088405 + 1`; seeds A and B; quality
  q = byte × 1/255 in double, stored as float): a twist per part from B;
  per part and deform value = (2B−1)·blend + (1−blend)·(2A−1); per part a
  presence roll (the exclusion bit is the part index modulo 32) and a
  weighted pick among variants whose tags agree with earlier choices. The
  first part offering a tag owns it (the whole owner table starts unowned).
  Then per area a weighted material roll (weights 100/50/20/10 by class,
  scaled by distance from q to the candidate's level, rounded half to
  even) with a finish (nibbles 3/7/11/15) and, unless the candidate fixes
  it (mode 5), a colour by the material's colour class, the item
  randomiser's colour roll with grade round(4q): cloth any family at shade
  7 − round(6r²); leather a cloth colour when r ≤ (grade − 1)/4, else a
  family 5–11 at 7 − round(4r²); steel 3, iron 4; painted steel the item
  palette's metal colour; class 6 (material 15) one of `0x4B`, `0x53`,
  `0x3B`; copper one of six; wood a family 5–11 at 7 − round(7r²); painted
  planks any family and shade; gems and resin none. The area keeps the
  colour on the lifted palette ramp, stored as bytes. A patterned
  candidate appends an emblem layer: three draws pick an emblem id from
  40, a new family and a shade near the area's.
- **Stored choices** (section flags): a fresh item with seed A set and seed
  B zero copies A into B, marks the section `0x30000000` and rolls the
  choices from A (three spare draws per present part so the area colours
  land where a full roll would; the first three areas take their first
  candidate's material and a random colour). The three generation paths
  then are: no flag, a full A+B roll which stores its choices and sets
  `0x10000000` when A is set; `0x10000000`, replay the selection and areas
  while twists use B and shapes blend A and B; `0x20000000`, replay the
  stored choices but roll shapes and twists from A alone.
- **Assembly**: components (`Components.rpk`, +Y along the weapon, cm;
  the first node, a mesh) are placed in recipe order on their nearest
  chosen ancestor. Attach mode 0 adds the parent variant's child Y offset
  ×0.1 (in double), modes 1–3 add the parent's box minimum and 4–6 its
  box maximum: the component node's stored box, before any deform. The
  ancestor's scale and band deforms then move the attach point; a parent
  deform flagged `0x40000` also deforms the child's mesh, and a scale
  flagged `0x10000` scales the child. The point is scaled by the parent's
  recipe scale and moved to the parent's position; a part without a
  parent stays at its recipe position. The part's matrix turns X, then
  Z, then Y, and takes the part's (possibly scaled) scale; twists
  accumulate down the ancestor chain. The bend uses the engine's packed
  sine and cosine (one reduction by round(|x|·4/π) quarter turns, so it
  drifts a few ulps from the exact functions) and treats a pivot or a
  radius up to `0x33FFFFD7` as zero.
- **Paint**: each assembled part is a region of the factory's atlas (its
  atlas rectangle times the atlas size). Its masks `<model>m1`..`m4` take
  the materials of the areas the variant names (material 0 paints as
  SteelGen), the area's lifted colour bytes ×1/255 and ((finish & 0xF) +
  1)/16 as the wear weight (`MatCol.w`); the first missing mask ends the
  list. The job's condition is the item's quality, damage (as 2/(2 − d)
  − 1) and dirt bytes /255. Material-scale textures repeat once per as
  many atlas texels as the material's first colour texture is wide,
  damage-scale ones once per 1024; four offsets from seed B,
  round(512r)/512 − ½ each, shift them. Not reproduced yet: texture-only
  overlay parts (part flag 1) and the mask-gated shade of parts flagged 2,
  emblem layers, the texture-only deform's `UVScale`, and the low-quality
  half-size atlas.
- **Items** name a factory by a model name starting with `@`. Their weapon
  section (`0x4000`, 96 bytes) holds flags, seeds, quality, damage, dirt,
  4 material slots (material, finish, colour, mode), stats, and the stored
  choices at +0x50.
- **Striking extent**: a role 2/3 head (or a role 6 part) stores its
  assembled Y range over the root's top in bytes `0x35`/`0x36` (or
  `0x3B`/`0x3C`), ×200 rounded. The stat-only refresh does not assemble,
  so it keeps the template's bytes.

Exarust follows all of this (`exa_formats::factory`, `exa_assets::weapon`).
Not yet: the shared atlas and its UV transform (each part keeps its own
textures), the merged mesh and its rebuilt normals, the striking extent's
denominator (Exarust takes the highest placed box top for the merged
mesh's box), the TTetraRBD body and its impact sound, and icon-only
builds. Stale state the original reads from its cached factory (an
unrolled area's previous material, a replayed area's previous mode and
emblem record) is replaced by the defined values in the code.

## Weapon stats

Status: **verified** (REA on the builder, the factory stat pass and the
condition step). A reference implementation reproduced bytes `0x30..0x3F`
of all 143 factory weapons in a real save, byte for byte. Exarust:
`exa_formats::weapon_stats`.

**Weapon section bytes** (`0x4000` body):

| off | meaning |
|---|---|
| `0x30` | weight, 0..100 |
| `0x31` | balance, 0..100 |
| `0x32` | impact adjustment (127 neutral) |
| `0x33` | thrust |
| `0x34` | striking side's type code |
| `0x35`, `0x36` | start and end of the striking part, ×0.005 of the length |
| `0x37`, `0x38`, `0x39` | slash, crush, pierce |
| `0x3A..0x3F` | the same block for the flipped side (e.g. a falchion's back) |
| `0x40` | adjustment, 0x80 neutral |
| `0x41` | hit sound class |

The section's first word holds the flags:
- kind (bits 0–3): 1 bladed, 2 hafted head, 3 stick, 4 flail;
- hands (bits 4–5): 0 unarmed, 1 one-handed, 2 two-handed, 3 polearm;
- haft material (bits 16–17): 1 wood, 2 metal, 3 other.

The low 16 bits also pick the holder's moveset (REA on the routine run
after every equip change; [PLAYER.md](../PLAYER.md), Moveset from held
items): by the hands bits, and among two-handers "Greatsword" for exactly
`0x521`, "2H Light" when bit 0 is set, else "Two Handed". The installed
object database's 208 weapon items give "One Handed" 136 times, "Polearm"
32, "Two Handed" 19, "2H Light" 18 and "Greatsword" 3; none lacks hands
bits. The torch, "Torch A01", is a one-handed stick (`0x01000013`). The
fists' built-in records have no hands bits (flags 0, or `0x40` for the
two clawed ones).

**Factory weapons** start from their template item's bytes.
- Each chosen variant adds its slash, crush, pierce and thrust (variant
  +0x1A, i16 each, low byte, wrapping). It also sets the type code from
  its flags (>> 4 & 0xF). Role-6 parts fill the flipped block instead.
- Each deform with value x = v·stat weight (deform +0x14) scales bytes by
  role through F(x) = 1 + x·amount, or 1/(1 − x·amount) when that is ≤ 0,
  and G(x) = 1 + x, or 1/(1 − x) when x ≤ 0 (the second helper ignores the
  amount). Terms marked (d) are computed in double.

| role | deform | effect |
|---|---|---|
| blade | type 1, Y | weight ×F, balance ÷F |
| blade | type 1, X | weight ×F, balance ÷F, slash ×F, thrust ÷F |
| blade | type 3, Y | balance ×(F/2 + ½), slash ÷ the same, thrust ×F |
| blade | type 4, X | one-sided: thrust ÷(G/4 + ¾); else thickens: ×(G/2 + ½), thins: ×(G(−x)/2 + ½) |
| haft | type 1 or 2, Y | f = 0.4G + 0.6 (d); weight ×(f/2 + ½), balance ÷f |
| haft | type 1, XZ | weight ×(0.1G + 0.9) (d) |
| guard | type 1, Y | thrust ×F |
| head, role 2 | type 1, any axis | f = F, halved and raised by ½ on axes 1–6; weight ×(f/2 + ½), balance ÷f |
| head | type 3, Y | f = G(−x/2), then f/2 + ½; weight ×(f/2 + ½), balance ÷f |
| head | type 5, all or XZ | h = 0.6G(x/2) + 0.4; pierce ×h, crush ÷h (d) |
| head | type 5, Y | f = 0.4G(x/2) + 0.6 (d); weight ×(f/2 + ½), balance ÷f |

  Role-6 parts' deforms change nothing.

- Each rounded product is stored as a byte, so values above 255 wrap
  (the condition step clamps instead). Weight and balance are then capped
  at 100. Variant flag `0x800` removes the flipped side.
- Factory items then scale by condition c = (1 − 0.4d)(0.8 + 0.5q), with q
  and d the quality and damage bytes /255 (the damage term and the product
  in double, the quality term's product in float, c stored as float):
  - slash, pierce and thrust ×c, in float;
  - crush ×(0.7c + 0.3), in double;
  - each rounded to the nearest (half to even) and stored as a byte, so a
    product past 255 wraps, then capped at 160.
- The head's striking extent (`0x35`/`0x36`) comes from assembled
  geometry (Weapon factories); the stat-only refresh keeps the template's.
- Materials and dirt do not enter the stats. There is no physical
  material table.

The record a held weapon fights with is built from these bytes: see
Weapon record below.

## Weapon record

Status: **verified** (REA on the record builder, the holding pass, the
length and sweet-spot walks, the mass spread and table, the equip
refresh, the unarmed set-up, the shield record and its attach, the step
reach; corpus below). Exarust: `exa_formats::weapon_record`
(`exa_combat::damage::WeaponRecord`).

Every body that can strike carries a record of 28 elements W0..W27: a held
weapon, a hand, or a shield (a short record of its own). Lengths are in
cm. Below, w and b are the weight and balance bytes /100, g the holder's
armour encumbrance, s its character size, u the share of the held
weapons' weight a special worn item takes away (0 for everyone else).

**Build**, from the section (the flipped block for `0x34..0x39` when the
weapon is flipped):

| element | value |
|---|---|
| W0 flags | the section's flags & `0x0603FFFF` |
| W1 type code | byte `0x34` |
| W3 | the start below, 0.7 when under 0.15, at most 0.99 (readers not found) |
| W4, W5 striking part | bytes `0x35`, `0x36` × 0.005, of the last part's length |
| W6, W7 | w, b |
| W10 impact | (1 − b)·0.6 + 0.4w + 0.1 (0.4w + 0.1 above balance 100) + (byte `0x32` − 127)/100 |
| W11..W14 | slash, crush, pierce, thrust: 0.5·(x/100)^0.7, 0 for a 0 byte (double precision) |
| W16 handling | 0.8 − ((1 − u/2)·w + 1 − b)/8; the body turns at it × B·(1 − 0.25·progress)·P while attacking ([PLAYER.md](../PLAYER.md), Combat mode) |
| W21 length | armed: the box top (centre.y + half extent.y) of the weapon's node plus that of every rigid node down its first-child chain, stopping at the first node that is not rigid; unarmed: across the hand, \|centre.x\| + half extent.x |
| W22 sweet spot | armed: whole parts while the next one down the chain is rigid, then (W4 + 0.6·(W5 − W4)) of the part it stopped at; unarmed: W21 |
| W23 (unarmed) | the end byte as a number (80 for the fists); unused for reach |
| W24 second hand | (byte `0x40` − 128)·0.00156 + 1: where the second hand holds a haft (a two-hander's −30·W24 cm, a polearm's −40·W24) |
| hit sound | byte `0x41`: `WH_SwordA`, `WH_MaceMA`, `WH_AxeA`, `WH_ClubMW`, `WH_MaceMA`, `WH_ClubMW`, `WH_WoodA`, `WH_ClawsL`, `BdyCol` |

A node is rigid when its mesh body is followed by a collision body chunk
(`0x3DD0B001`, [RFC.md](RFC.md)). An assembled factory weapon is one part
in the original, so its length is its assembled top. Corpus: the box of
every one of the 103 plain weapon models of `Objects.rpk` tops at its
highest vertex within 0.05 cm; "Torch A01" is 58.3 cm (its shadow and
flame nodes are not rigid), "Flail A" 84.4 cm and "Ogre Flail" 172.0 cm
(their links are rigid).

**Flip side.** The flipped block's type byte (`0x3A`) is zero when the
section defines no second side. The flip command refuses a weapon whose
`0x3A` is zero (or that has no weapon section), and a character in the
switching condition; a flipped record takes code, striking part and damage
from the flipped block (`exa_formats::weapon_record::Section::can_flip`).

**Holding pass**, at the build and on every equip change:

| element | value |
|---|---|
| W8, W9 held weight and balance | w, b; with the half-sword hold (2 − w)·w and 0.7·b (what sets it is not found); W8 × (1 − u) |
| h | (1 − W9)·0.6 + 0.4·W8 + 0.1 (0.4·W8 + 0.1 above balance 1) |
| W17 recoil | g/2 + h: a blocked striker recoils by v·W17; the recovery after a released swing follows it, and the attack cooldown is round(300·W17) + 300 ms of the primary striker (400 ms with none) |
| k | ((1 − g) + W9)·g/2 + W9·(1 − g) |
| W18 swing shaping | 1 − k (the swing clock slows away from its middle by it) |
| W19 blend | round((W8 + 1 − k)·10), an integer: attacks end into the stance over 2·W19 frames |
| W20 arm stiffness | 0.7 − 0.3·W17; two-handers (hands bits 2) out of the half-sword hold × (0.7 − 0.2·W9); greatswords (`0x521`) × 1.5; unarmed 0.5/s |
| W15 swing speed | (1 − W8)·0.15 + 1.05; × 1.2 for the human fists (no hand-class bits) |
| W27 AI swing turn | min(0.4·W18 + 0.3·W8 + 0.4, 1.5); unarmed × 1.5 for claws, × 0.5 for fists |
| W25, W26 swing volume and pitch | up to 124 cm 1.5·√((250 − L)·0.004) and (250 − L)/180, else 1 and 0.7; W26 × (1 + d/2), d = W9 − W8 halved when negative; unarmed W25 × 0.7·√scale, W26 × 0.6/scale |
| W23 reach | armed: 60·s + W22 |

Precision (REA, the pass's listing): the half-sword's 0.7·b, h (its
0.4·W8 + 0.1, then (1 − W9)·0.6 added), W20 (0.7 − 0.3·W17 and the
two-hander's factor), W15 (0.15 and 1.05, the fists' 1.2) and W27
(0.4·W18 + (0.3·W8 + 0.4)) multiply in double and store a float; the
unarmed W25 is √scale·(0.7·W25) and W26 0.6·W26/scale, both in double;
over 124 cm (not a non-number) the long weapon's 1 and 0.7, and over a
balance of 1 (not a non-number) h drops the balance term. W19 rounds to
the nearest, ties to even. A two-hander of w 0.37, b 0.63, 110 cm (sweet
spot 80) held at g 0.3, s 1.1 has W17 0.62, W18 0.3595, W19 7, W20
0.29503599, W15 1.1445, W27 0.6548, W25 1.1224972, W26 0.87888891 and W23
146 cm (bit-exact in `exa_formats::weapon_record`'s tests).

Worked example, "- Sword A" (90 cm): W15 1.125, W17 0.42, W18 0.2, W19 7,
W20 0.574, W22 57.6, W23 117.6, W25 1.2, W26 1.022, W27 0.63. With g 0.5:
W17 0.67, W18 0.275, W19 8.

**Mass** (simulation units): M = heavy·w + (1 − w)·light by hands and size

| size | one-handed | two-handed | polearm |
|---|---|---|---|
| normal | 0.16 – 0.40 | 0.20 – 0.50 | 0.20 – 0.50 |
| ogre (`0x2000000`) | 0.32 – 0.80 | 0.50 – 1.00 | 0.50 – 1.00 |
| demon (`0x4000000`) | 0.50 – 1.20 | 0.75 – 1.50 | 0.75 – 1.50 |

The part tree redistributes in two passes, parent before children
(`exa_physics::weapon_mass`). The holding pass hands it the weapon's table
mass; Exarust runs it when a weapon is equipped, over the rigid nodes of its
model (`exa_character::weapon_mass_of`, kept as `WeaponMass` for the body
solver). Each node's initial mass is the reciprocal of the mobility the body
solver gives it, the same word the shape builder stores as its
`Shape::mass`; a factory weapon's assembled model has no rigid node and
starts as the original's one merged part. After the two passes the
holding pass divides the root part's mass word (the weapon part's own) by
the holder's size; its inverse word keeps the distributed value. A saved
mass of zero is replaced by the
live mass and then left alone. The live mass becomes 1.25 times that
saved mass and the inverse word becomes its reciprocal; the walk counts
the parts and sums those reciprocals. The second pass writes both words
with the reciprocal of (0.5/count + (inverse/sum)·0.5) · scale, where
scale is the caller's factor. A saved mass that is already set is the
one inflated. A zero inflated mass
divides the way a single-precision division by zero does.

**Equip refresh**: each held record is held again with the character's g
and s; the character's reach is the largest W23; W19 = round(W19 × race
`0x90`) (0.5 for the five golems, else 1) and W27 × race `0x88` (0 for
Sir, the harvester, the demon and four golems; 0.5 for the ogre, the
gobbler and the golem suit; else 1); the AI's thrust preference is
round(W14/(W11 + W12 + W13)·5) + 1 capped at 9, 10 without edges, 0
holding nothing. Unarmed, the reach is race `0x84` (human, skeleton,
ancient, manipulator, wraith and golem suit 50 cm; ogre, ghouls, demon and
the other golems 100; Fael 130; Sir, gobbler, harvester 150) and the
hands weigh twice their part's mass (four times with heavy
claws). Race index 6 and 7 (the ghouls) swing with W20 = 1; race index 5
(the Fael) turns half as far (W27 × 0.5).

**Fists and claws.** Unarmed, both hands are held, each with a built-in
section: the right hand's by race index (8 heavy claws; 6, 7 claws; 4, 5
heavy claws), the left hand's by the race's word at `0x58` (`0x100` claws,
`0x20100` heavy claws), else the human fists. So the Harvester (index 10,
word `0x20100`) strikes with a right fist and a left heavy claw.

| section | flags | balance | impact | type | start = end | slash, pierce | `0x40` | sound |
|---|---|---|---|---|---|---|---|---|
| fists | 0 | 100 | 160 | 1 | 80 (0.4) | 0, 0 | 0 | 8 `BdyCol` |
| claws (ghouls) | `0x40` | 100 | 180 | 1 | 60 (0.3) | 60, 40 | 0 | 7 |
| heavy claws | `0x40` | 100 | 200 | 1 | 150 (0.75) | 60, 40 | 0 | 7 |

A human fist is 17.66 cm (the hand node's box), with impact 0.43, W15
1.44, W17 0.1, W18 0, W19 0, W20 0.5, W24 0.80032.

The hands themselves strike with these records: a fist's contact is
measured along the hand's x axis toward its box (negated for the right
hand, whose box lies on −x), and one at or past W21 is ignored. With the
fists, claws or two weapons (weapon class bit `0x80`) an attack strikes
with the motion's hand only: the right (first held) in the main side, the
stab and the overhead (both hands in a clawed overhead, class `0xC0`), the
left (second held) in the other side, which then becomes the primary
striker. Equipping the fists leaves the character without a primary
striker until its first attack (a 400 ms cooldown); equipping a single
weapon makes it the primary. In Exarust each hand gets a striker segment
from its origin across its box, as long as W21; a contact at its far end
counts 0.1 cm inside it, since the segment ends on the box's face where the
original's hand body is met within it ([PLAYER.md](../PLAYER.md), Combat
mode).

**Shield record**, item section `0x8000` (22 items): weight, block and
bulk = bytes 4, 5, 6 × 0.01, each in double precision then stored as a
float (a round shield 0.5, 0.58, 0.6; a buckler's weight 15 is 0.15, where
single precision gives 0.14999999). On the part those three floats fill a
16-byte block (`ShieldBlock`). The block is created with its first word
`0x80000000`; an existing block keeps its first word. With no section
body, the item's `0x8000` section object is used and the body is read
eight bytes in. A missed lookup leaves the three floats unwritten, and so
does a body that does not hold bytes 4, 5 and 6 (the original reads them
with no length check). In the off
hand it gives parry effectiveness 0.5 + 1.5·(1 − weight)² (weight/2 with
the Mobility technique), a block leak of 1 − block/2, a stamina cost per
block of the leak (× 2/3 with Endure), and adds (2 − u)·bulk/4 to the
encumbrance.

**Step reach**: 80·√scale·(1 − 0.3·feral, when the feral level is over
0)·(1 − 0.2·g), in double precision (a feral level of 0.84 gives 59.84 cm,
single precision 59.840004); the long steps and the footwork are measured
in it. Exarust has one implementation (`weapon_record::step_reach`), which
a character's stats read in metres.

In Exarust every held weapon and fist is held by its character's own
stats (`CharacterStats::holder`, [RFC.md](RFC.md), Stats derived at
spawn): s its size (0.95 to 1.05 for a human by height), the scale its
body scale and g its encumbrance ([Encumbrance](#encumbrance)); no worn
item makes held weapons weightless yet (u 0: the shield-lightening
gauntlets are unpowered in every shipped record). The race fields come from
the race's decoded stats (`races.rdb`, `+0x58`, `+0x84`, `+0x88`, `+0x90`,
above), kept with the race index on each placed character's body; the
player and the duellist are human. A duellist guards beyond four reaches
([PLAYER.md](../PLAYER.md), The combat stance).

## Armour

Apparel item body (`0x1000`) fields:
- `0x150 + 4·(bone − 1)`: coverage percent (u32) per bone;
- `0x1A0`: padding ×0.005;
- `0x1A1..0x1A3`: slash, crush and pierce ×1/140;
- `0x1A5`: encumbrance ×0.0025 ([Encumbrance](#encumbrance));
- `0x1A6`: hit sound class (0 flesh, 1 cloth, 2 leather, 3 mail, 4 plate,
  6 golem);
- `0x1A8`: armour type, `0x1A9`: wealth variance (bits 2–6 in 31sts, bit 7
  the sign), `0x1AA`: armour variant (1–8; `0xFF` until rolled), `0x1AC`: the
  paint seed (u32; 0 until rolled);
- `0x1B0..0x1B3`: quality, damage, dirt and their packed spreads (the
  "unknown" fourth byte of the appearance block: quality in bits 0–2,
  damage in 3–5, dirt in 6–7; codes 0 and 1 are eighths, the others
  quarters less one; non-zero means the copy still rolls);
- `0x214`: special kind (1 sees in the dark: the player's own light turns
  on while one is worn, the three night-vision helms "UW Helm NV", "UCA
  Helm A" and "UGA Helm FB" of the shipped database, established by REA on
  the light switch, which reads this byte, and a corpus scan; 4 the golem
  suit, 6 gauntlets that lighten a held shield; 2, 3, 5, 7 other item
  effects: the strength bracelet, the healing mask, the null helm and the
  spirit necklace carry them);
- `0x218` bit 0 and `0x219`: a power crystal is in and its charge (0xFF
  spent): the special item is powered;
- `0x220`: the articulated set a piece belongs to, the item id of the set's
  central piece (14 sets, e.g. 445 for the "UBE" plate), 0 for none.

An item made from a template takes the template's bytes scaled by its own
condition, c = (1 − 0.4·d/255)·(0.8 + 0.5·q/255) with q and d the
appearance block's quality and damage: padding and resistances × c,
rounded (half to even), at most 160; the coverage display byte × (0.3c +
0.7); encumbrance ÷ ((1 − 0.15·d/255)·(0.85 + 0.3·q/255)), rounded. The
armour variant then scales the bytes once more, without a cap (the byte
wraps): 1 resistances × 0.9 and encumbrance × 0.8; 2 × 1.1 and × 1.16; 3 ×
1.2 and × 1.1; 4 slash and pierce × 1.35 and × 1.06; 5 padding × 1.4,
crush × 1.2 and × 1.06; 6 padding and crush × 1.3, slash and pierce × 1.1
and × 1.16; 7 and 8 nothing. A copy's price is the template's ÷ (2q₀² +
0.5) × (2q² + 0.5)(2(1 − d)² + 0.5)/2 in the compact price word
([Containers and random tables](#containers-and-random-tables)). The
armour pass then reads the bytes as the item stores them: the level
databases' instances store them scaled already (of those whose condition
changes their encumbrance byte, 381 store it scaled and 1 as the template;
of their protection bytes 2200 against 4), and a record wearing a game
template's id reads the template's bytes. Exarust reads what each item
stores and scales nothing again (`exa_combat::damage::armour_layers`).
Races add a natural inner layer: `races.rdb` coverage at `0x21C`,
protection at `0x11C`. Skeletons, golems, demons and the Fael head have
one.

Each body part keeps two layers, filled in equipment order. A new item
folds the old outer layer into the inner one:
- each layer's effective coverage is γ(c) = c, or c + 1.75(1 − c)(c − 0.5)
  above ½;
- padding adds as γ-weighted;
- resistances combine as s0′ + (1 − s0′)·γ1·s1, with s0′ = γ0·s0;
- the inner layer's coverage becomes 1.

An item's coverage of a part is its percent × 0.01 (in double, stored as a
float); with the Deflection technique learnt it is c + 0.1 up to ½, else c
+ 0.1·(1 − c) + 0.05, at most 1 (30 % covers 0.4, 75 % 0.825, 95 % all).

The 19×8 slot table at `0xA8` is not read for armour.

## Encumbrance

A character's one weight stat, implemented in `exa_formats::encumbrance`.
The original's armour pass sums it at spawn, after the character's init and
before the reach pass (the held items, a shield among them, are attached
before both), and again on every equipment change. Each step runs in double
and is stored as a float:
1. for each worn item in the record's order whose id finds an item with a
   body (an actor's local ids name its own database, a level's its own, the
   rest the game's): an unpowered golem suit adds 2.0, powered gauntlets of
   kind 6 make the held shield weightless (unpowered ones make it count
   again), then + encumbrance byte × 0.0025;
2. each articulated piece (set word ≠ 0) whose covered bones touch another
   articulated piece's takes 0.025 off, once: piece A touches piece B when
   A's bones meet the bones B's bones neighbour, or the other way round. The
   set ids are not compared, so pieces of different sets count too. The
   neighbours of each bone 1–18 include the bone itself (pelvis: femurs and
   lumbar; thorax: lumbar and head; …); the table gives the left humerus
   (15) the right shoulder (11) instead of the left one (14);
3. − 0.3 × the highest Manoeuvring in Armour tier learnt (0–3);
4. below 0.001 (negative included) it is 0;
5. with a shield held: (2 − weightless) × bulk / 4 (the shield section's
   byte 6 × 0.01), less 0.3 with the Mobility shield technique, is added
   when positive. The shield is the last of the first two held items that
   has a shield section and no weapon section (the left hand's light
   section would be taken first; none of the 22 shields has one);
6. at most 1.

Shipped values: the duellist's leather jacket (10) and full helm (60) give
0.175; a tourney plate harness, helm and leggings of one set (120, 80, 80,
all touching) 0.625. Of the story levels' 451 living characters 386 weigh
nothing (many learnt Manoeuvring III), 27 hold a shield, Elestan weighs
0.77 (0.47 of armour and a round shield's 0.3) and Demian 0.67; the arena's
opponent07 0.475 and opponent13 0.07 (Manoeuvring II takes its armour off,
its buckler's 0.07 stays).

What it slows, by encumbrance e (the readers' own precision):
- the step reach: 80 cm·√scale·(1 − 0.3·feral)·(1 − 0.2·e)
  ([RFC.md](RFC.md), Stats derived at spawn);
- the run blend: at most 1 − 0.3·e, 1 − 0.4·e with the sprint latch (Exarust
  applies the first to the player's run, [PLAYER.md](../PLAYER.md));
- the sprint gait's clip clock: × (1 − 0.1·e);
- the body's turn toward its heading (out of the free walk): × (1 − 0.6·e);
- the alignment a standing body settles to: 1 − e;
- a long step: (1.5 − 0.3·e) step reaches (the player's held step keys, the
  feet's long steps); the AI's long footwork step (1.6 − 0.3·e);
- the get-up clip clock: (1.5 − 0.5·e) × the clip-rate base;
- the held weapon's record: the holding pass takes e as its grip input
  (recovery, swing clock), and so does the unarmed hands' set-up.

Exarust sums it at spawn for every placed character (`exa_world`) and for
the player's and the duellist's outfits (the first painted item worn as
each model); `exa-tool characters` prints it. The step reach, the player's
run blend, a held step key's long step and the held weapons' and fists'
records read it; the other readers take it when they are ported.

## Damage

Implemented in `exa_combat::damage`.

The rules below are the original's damage routine, re-read in its listing
(REA, Ghidra) with each step's precision: "double" marks a constant the
routine multiplies in double precision before it stores a float. A Python
model written from the listing gives the same numbers as `resolve` to a
millionth on every case its tests pin (a cut on a bare and a plated chest,
a slow overhead, a stab, a mace's haft and head, a punch, claws, the
victim rules). ε below is the routine's epsilon, 1.19e-7 (bits
`0x33ffffd7`).

**Speed and impact**
- Speed v of the struck point (cm/tick): over 30 it is 30; under 10 it is
  softened to 10·(2 − s)·s² with s = 0.1v (double); an overhead then
  ×1.5. At ε or less the blow has u = 0, v = 0 and impact ε; else
  u = 0.05v (double).
- Swing impact I = (W10 + 0.3)·1.1·exp(ln u·(1 − W10/2)), in double; stab
  impact (W10 + 0.5 + W6)·u·0.3 (the 0.3 double). Both × (Force Strikes
  s/2 + 1) × (1 − Force Shell); neither power exists in Exarust, so ×1.
- The hit point p = along/W21: under 0 it is 0, at or past 1 there is no
  hit (a non-number goes on).

**Zones** (what struck, in the routine's order)
- A stab (the stab motion, unless the attacker's race walks on four
  limbs, which resolves it as a swing): the tip factor 1 within 7 cm of
  the tip, 1 − (t − 7)/7 to 14 cm, 0 beyond; the lunge
  a = 1 − 2.8·(1.4 − 2·clock)² (double), at least 0, × the powers; the
  stab energy T = tip·((0.3·W6 + 0.25)·(I + a)) in double, ×1.5 when the
  weapon is two-handed or a polearm; the impact at the hit
  (0.25·W10 + 0.75)·T. No slash or crush. A body hit, zone p, bite
  smoothing 0.
- A striker hanging from another striker of its weapon (a flail's link):
  the record's slash, crush and pierce in full, the impact as it is, a
  body hit, zone 0.2, smoothing 0.5. Exarust's weapons are one body.
- Unarmed (the attacker's weapon class has no hands bits): the shares in
  full, the impact as it is, not a body hit, zone 0, smoothing 0.5.
- Armed: past the striking part's end (p > W5) the on-target share
  z = max((20 − d)·0.05, 0) (the 0.05 double) with d = (p − W5)·W21 cm,
  the tip zone q = (p − W5)/(1 − W5), ζ = 1 − 0.75q², the draw share
  q·(1 − 10·(0.8 − clock)²) (double) when positive, an edge hit. Before
  its start (p < W4) the haft: z by d = (W4 − p)·W21, ζ = 1 − (W4 − p)/W4,
  a body hit. Else z = ζ = 1, a body hit. Then by kind: bladed, past 0.6
  the tip zone q = (p − 0.6)/(1 − 0.6) (ζ 1 − 0.75q², draw q, an edge
  hit), else r = (0.6 − p)/0.6, ζ = 1 − (2 − r)·r²; a stick likewise at
  0.8 ((p − 0.8)·5 and (0.8 − p)·1.25 in double) and never a haft; any
  other kind whose type code is 2 likewise at its sweet spot
  W4 + 0.6·(W5 − W4) (double), ζ = 1 − (τ − p)/τ before it. Slash, crush
  and pierce are the record's × z; crush gains (1 − z)·k (double) with
  k = 0.15, or for a haft hit 0.1, 0.2 for a metal haft (flags 0x20000),
  0.15 for the third kind (0x30000), whatever z. The haft's sound plays
  only under z = 0.5. The impact at the hit is I·ζ.
- Unarmed, the impact at the hit becomes 0.6·√(3·I) (the 0.6 double)
  above ε.
- The hit sound: volume (I + T)·0.6 + 0.025 (double), pitch 0.5·I + 0.5;
  the weapon's sound at volume × (½ unarmed) × the attacker's body scale ×
  (0.6·Rc + 0.7) (double) and pitch × the same, its haft's class sound
  instead for a haft hit.

**Armour**
- A blow on a part draws one roll r from the generator (a blow on a shield
  or a weapon draws none and meets no layer). The outer layer, then the
  inner: weight 1 when its coverage is above r, 1.75·(c − ½) when above ½,
  else the layer is skipped.
- Padding A += pad·w; Rs += (1 − Rs)·(slash·w); Rc likewise; Rp +=
  (½r + ½)·((1 − Rp)·(pierce·w)).
- Layer sounds: the outer rings at its weight; the inner at none when its
  class rings as the outer's, else at (1 − 0.8·outer) (double) × its
  weight; each at 1.5·volume × weight × its class's loudness (flesh 0.5,
  the rest 1), pitch 0.4·pitch + 0.7 (double), above ε.
- A blow on a part sets the victim's blow sag to min(2·I, 1) (it fades
  0.01 a tick, [RSQ.md](RSQ.md)).

**Through the armour**
- τ = 1.25·I. J = I − 0.2A (double); if positive, J/((A² + 1) + A), else 0.
  A stab's T /= ((A/2)² + 1) + A/2 when positive. The bite β = 0.1·I
  (double).
- Stab: pierce = W14·1.58·√T (double) under T = 2 or when T is no number
  (0 when J ≤ ε), else W14·1.58·√2; β += max(2·pierce − Rp, 0).
- Swing, crush C > 0: C·1.58·J^(2 − C) with the engine's fast power (0
  when J ≤ ε); τ += 0.75C; β += max(0.5C − 0.25Rc, 0);
  C = (1 − 0.75Rc)·(C − 0.1Rc) (double), at least 0.
- Swing, slash S > 0: k = S/(10·S·Rs + 1); chop = ((0.8 − k)·I² +
  2.5·I/(I + 1 − k))·k − 0.2Rs (double; 0 when I ≤ ε); β += max((1 −
  draw)·chop, 0); drawn = ((1 − k)·u + √u)·k − 0.3Rs (0 when u ≤ ε);
  β += max(½·drawn·draw, 0); S = draw·drawn + (1 − draw)·chop, at least 0;
  τ += 0.75S.
- Swing, pierce P > 0: P·1.58·J^(1 + P/2) by the fast power (J = 0 gives
  0); β += max(P − 0.5Rp, 0). Then β ×= ½.
- Both: J = J/((2J − 0.2)·A + 1) (double), at least 0; τ += 0.75P;
  P −= 0.75Rp, at least 0.
- The fast power x^y = exp(y·ln x) with the engine's fast logarithm and
  exponential (`exa_combat::fast_math`): ln reads the exponent bits and a
  mantissa polynomial (ln 0 = −∞), exp rebuilds float bits (exp −∞ = 0).

**Outcome**
- Penetration P = 0.5·max(J − 0.8, 0) + ((S + P) + C) (J − 0.8 double).
- The shove takes J as it is: over ½ the knockback step
  ([PLAYER.md](../PLAYER.md)).
- Readiness g: P, J and T × (1 − 0.5g).
- Part multipliers (damage/stun): head 1.6/1.4, thorax 1.2/1.2, lower legs
  0.6, feet 0.4, forearms 0.5/0.75, hands 0.3/0.6, others 1.
- Daze: D = 0.75P + 1.25J; daze = √D · (stun·0.5·0.25/max health) above ε,
  else 0. The daze's stall factor (which a struck attacker's swing takes)
  is 0.25/max health, × (D/τ′)² with the Security perk (τ′ = (1 −
  0.5g)·τ, 1 when τ′ ≤ ε), halved under a potion. A steadfast body (rule
  bit 0x40: the big races and golems) is dazed a quarter as much and its
  factor is 0.
- Lasting = P/(1 + P); the wound R = lasting × the part's damage; red =
  0.75·R; yellow = 0.75·(T/2 + ((P − lasting)·2 + J) × the part's damage).
- Maximum health is 0.25 × the race's factor (`races.rdb` `0xE4`: human 1,
  ogre 6, golem 2, …) with the init's race-byte rules, and health starts at
  the record's saved pools ([RFC.md](RFC.md), Stats derived at spawn).
- Applying a blow: see Applying damage below.
- A blow on flesh draws one more number from the generator and stamps
  the struck part's sound clock (`exa_physics::part::part_sound_ready`):
  the game clock plus 300 ms, less a draw below 150. A blow the body
  then turns still stamps it. A shield's first ring in a swing draws
  the same number and stamps that same clock.
- A body whose state flag 0x400 is set turns the blow: nothing is dealt
  (no push, daze or wound) and the striker recoils whatever its size; the
  blow sag the armour pass set before the turn stays (what sets the flag
  was not traced).

**The striker after a wound** (`SwingState::after_wound`)
- Its swing loses the released bit and takes 0x724 (cleaving, recoiled,
  the clash, block and wound bits); its striker's window shuts.
- A striker of character size under 1.4 (double): a body hit with an
  impact at the hit under 2 follows through with gain 1 − I/2, a harder
  one cleaves on; an edge hit follows through with gain 0.5 when its swing
  is stalling, released, chained past a quarter or without Remise
  (0xA0012), else cleaves on. A larger striker follows a body hit through
  at 0.5 and cleaves on from the edge. The follow-through: strength
  √zone·k′ (body; 0 at a zone ≤ ε) or (1 − q)·k′ + q (edge), k′ =
  gain·W17·v·0.15, at most 1; the stall (1 − P)·0.05; the stance
  400·k′ ms on.
- A parried or shielded blow, or one a body turns, makes it recoil
  (strength and stall ½, the stance 200 + 40·v·W17 ms on); a guard does so
  only to a striker under 1.4. A parrying body whose state flag 0x400 is
  set is checked before the parry: any striker recoils and the parrier
  takes nothing (no lock, no attack-state clear, no sag).
- A weapon clash on a parrying victim has three branches, in this order:
  the victim is parrying — clear its attack state, lock it
  600 ms unless the Riposte perk is on, end the parry and
  sag its stance by min(0.5·I, 1); else the victim counters this very motion
  (the motion it counters equals the attacker's) with a weapon other than its
  primary (the struck body is not that weapon) — end the parry with no lock
  and no sag; else nothing on the victim. The striker recoils in the
  first two branches (size under 1.4) and the weapon's sound rings once a
  swing in all of them. Exarust has no weapon bodies to be struck, so the
  primary test is always false; the branch is `parry::clash_branch`.
- A recoil keeps its stance blend and arm stiffness (`RecoilRest`) on the
  swing until its stall ends. The stance that returns then starts with
  blend 40 and both arms' stiffness 0.2, which the next character tick
  eases back up; a released swing, a wound or a new attack drops the
  rest.
- A wound causes no damage over time. The body tick's one damage-over-time
  routine is burning (`exa_combat::burn`): the particle body-contact pass
  adds a fire's dose to the burn through the part's virtual slot 39, for a
  part whose bone index is above zero (`Burn::add_dose`), and the damage
  routine never calls it. A wound's blood (the queue, spray and weapon
  stain) is cosmetic.
- A clash and a block set the swing's weapon-sound bit 0x1000 (the
  weapon's hit sound once a swing); a block sets the shield-sound bit
  0x200 once a swing (a wound's bits already hold it) and then draws the
  shield's sound cooldown from the generator (300 − 0..150 ms) onto the
  struck part's sound clock.
- The routine runs per contact: each contact's striker is dealt right
  after its struck side, before the frame's next contact, so two fighters
  wounding each other in one frame take a follow-through before the other
  contact's stall.
- An armed bite above 0.1 would stick the weapon (`exa_combat::bite`);
  held items now have bodies, but the bite link is not wired to them, so
  nothing sticks.

**Applying damage** (REA on the damage call, its two callers and the
low-health check, constants read from the executable; `Health::take` in
`exa_combat::fighter`)
- The call takes yellow and red amounts Y and R (a weapon blow's yellow
  and red above, already × 0.75; an impact's 0.5·d and 0) and whether the
  damage is vital (a weapon blow always; an impact once its accumulated
  strength passes 0.6). In order:
  1. the regeneration rate is set to 0 (below);
  2. yellow −= 0.1·Y, or 0.05·Y while a potion works (state flag
     0x800000);
  3. with no lasting wounds (rule bit 0x8: feral humans, skeletons and
     ancients, [RFC.md](RFC.md), Stats derived at spawn) yellow −= 0.2·R
     and red is untouched; otherwise red −= 0.2·R and yellow ×=
     red_new/red_old, or yellow = 0 when no red is left (red may go below
     0);
  4. yellow still 0.05 or more: done. Otherwise, unless yellow is under
     0.01 **and** the damage is vital, the character lives and the
     low-health check runs (yellow is set to 0 if it was under 0.01): a
     second wind for one wearing the armour that grants it, else, for the
     player, the heartbeat ([UI.md](../UI.md), Heartbeat). A vital one
     under 0.01 kills: yellow 0, in the story red −= 0.1 × the maximum,
     and the collapse starts ([RSQ.md](RSQ.md), The collapse).
- Precision: each subtraction in double precision on the single-precision
  values (0.1, 0.05 and 0.2 are doubles), the red ratio in single, the
  0.05 test in single, the 0.01 test in double, so a yellow of exactly
  0.01 in single precision (0.0099999998) dies.
- Worked numbers: from full health (0.25), Y 1 and R 0.5 leave red 0.15
  and yellow 0.09; Y 0.5 and R 0.3 leave a human 0.152 / 0.19, a feral
  human 0.14 / 0.25, a human under a potion 0.171 / 0.19; a killing blow
  in the story takes a red of 0.2 to 0.175. In the installed story levels
  248 of the 451 living characters keep their red under every blow: the
  97 feral humans, 128 skeletons and 23 ancients (corpus test in
  `exa_combat::fighter`).
- The call also spares a character whose "no death" byte is set and calls
  a player hook on a killing blow; nothing in the executable ever sets
  either, so Exarust has neither. Story mode is the game mode of the story
  levels (practice and the arena clear it).
- Yellow under 0.01 on a living character (in double precision): the
  player's combat stance is the Defence stance ([PLAYER.md](../PLAYER.md),
  The combat stance); the stance by energy (the swing stall's, a cleared
  parry's, the AI's other branch) gives the Defence stance; the
  auto-parry clears any parry and starts none. Only `--health` makes it
  happen in Exarust today: the weapon blow is always vital.

**Regeneration** (REA on the body tick's prologue and health bookkeeping,
the damage call and the weapon damage routine, constants read from the
executable; a retail trace; `exa_combat::regen`)
- Each tick of a character not dead (a collapsing one included), after
  its posture and attack and before its clip advances: its rate r is 0
  while it attacks (condition 2) or while Cmbt - Idle plays (whatever the
  condition, so a weak fighter in it too); otherwise r rises by
  0.0003 × C a tick (in double precision), at most C = 0.01/83 (a float
  in the image, never written), and yellow gains √red × r (in float) while
  red is above 1.19209e-7. The ramp takes 3,334 ticks (40.2 s); at the cap
  a human with full red heals 0.005 a second, the whole bar in 50 s.
- r is set to 0 by every blow that deals health and by every weapon
  contact on a character, before the damage routine decides what the
  contact does; a burn that still has strength clears it in the upkeep
  stage, before this bookkeeping (`exa_combat::burn`).
- An active second wind, after the heal pool and before the rate ramp,
  adds the double 0.0003 to yellow and stores the float, whether or not
  the fighter attacks. The wind stays while the game clock is at or before
  its ready clock minus 180000 ms plus 3000 ms. The tick past that clock
  still adds, then the wind ends. Yellow bits 0x3d000000 become
  0x3d013a93. A quiet wind adds nothing. A dead fighter does not take
  this tick.
- Red heals only from the heal pool. A salve (operative kind 8) fills it
  when the body takes remedies: 0.2, or 0.1 when the section's value word
  is 50, added in double precision (`exa_combat::remedy`). An elixir
  (kind 14) sets the potion flag and stamps the world clock (the save's
  seconds plus game time) plus 1800 s; the body tick clears the flag once
  that clock is past the stamp. Either kind ORs `0xC80000`, whether or
  not the body takes remedies, into the item's flags when it is used
  directly, or into its container's inventory entry (the entry word less
  one) when that word is above zero; a zero word marks nothing. 0.25 × 5e-5 = 1.25e-5 a tick moves from the pool to red
  (a pool of 0.2 drains in 15,999 ticks, 193 s), and, dead or alive (the
  test comes before the dead test), for bodies whose rule bits hold 0x20
  without 0x100 ([RFC.md](RFC.md), Stats derived at spawn): 1.25e-5 a
  tick, 0.00104 a second. The ogre, Sir, the harvester and the demon have
  it; a golem suit has it only while it wears its core. In the installed
  story levels Sir and the three harvesters regenerate red; in the arena
  the demon.
- Then yellow is kept within red, and after it red within the maximum
  (so yellow follows a red cut to the maximum a tick later); at the end
  of the tick a yellow that compares below 0 becomes 0, and a NaN stays.
  Death sets yellow to 0 (the collapse
  regenerates about 0.0006 of it over its 250 ticks).
- A shield arm's stamina (0..=1) and its rate refill with the living
  body's health tick: while the stamina is below 1 minus the data epsilon
  the rate rises by the double 2e-6, narrowed, and is capped once that
  double exceeds 0.001; the stamina then rises by the rate. At 1 the
  stamina snaps to 1 and the block stamp becomes 0xffffffff; the rate
  stays at the cap. A block floors the stamina at 0, zeros the rate and
  stamps the game clock, so an empty arm reaches full rate in 500 ticks
  (6.0 s) and full stamina in about 1,250 ticks (15.1 s). The rule is
  `exa_combat::regen::ShieldStamina`, run from the health bookkeeping.
- The retail trace of the player after a fight: yellow 0.085, held for
  7.5 s in Cmbt - Idle, then in Idle 0.086, 0.090, 0.100, 0.110 and 0.121
  at 5.0, 9.6, 16.2, 21.1 and 25.3 s, 0.202 at 45.8 s; the rule with red
  0.203 gives 0.0864, 0.0902, 0.0997, 0.1100, 0.1209 and 0.2009. The
  Defence stance heals; Cmbt - Idle held yellow for 66 s in the same
  trace.
- Exarust: a light `--still-opponent --at 0,0,1 --health 0.085,0.203
  --press Tab@50 play arenao1` run, combat mode off at frame 50, logs
  yellow 0.086 at 4.4 s, 0.101 at 16.9 s, 0.118 at 24.4 s and 0.172 at
  39.4 s (the rule to the log's three decimals) and the full rate 40.16 s
  after the Tab. Since the health bookkeeping moved into each living
  character's tick (red regeneration in its upkeep stage, the rest in its
  attack stage after its posture; the dead and any character without a
  living body still once a fixed tick before the character ticks), the
  same run in the arena reaches the full rate at yellow 0.1755, the rule's
  value.

**What the struck character does** (REA on the damage routine and what it
calls)
- It starts no clip. The damage routine starts clips only through the
  striker's recoil, on blocked and clashing hits. The struck character
  takes the daze (energy × (1 − min(daze, 0.9)), at least 0.05), pain
  voices, and on J > 0.5 a knockback step: 2·(20·s + 10)·J cm along the
  swing flattened to the horizontal, s the Force Strikes level (0 without
  the power), plus 20 ticks of its chest's smoothed velocity, capped at
  its step length. Exarust starts that step when impact is over 0.5, and
  shoves the struck part by 10J cm/tick plus a tenth of the blade's
  velocity only when it is not.
- A character struck while swinging has its swing stalled. With D the daze
  (at most 0.9), f a factor of the blow (1 for a human in most blows),
  reach its attack reach (cm) and w its weapon's recoil factor,
  r = 4·D·f − (1 − D)·min(0.005·reach, 1)·w. When r ≥ 0.001 its swing
  strength P falls by r each tick and scales the swing's clock, and at
  min(80·P/r ms, the swing's remaining time) it returns to its stance,
  blended over a time from its weapon record. Exarust has no swing
  strength yet: a struck attacker returns to its stance at once and its
  blade deals no further hit.
- Rebalance, Defence stance, staggers and falls come from balance, not
  from the blow ([RSQ.md](RSQ.md), Falls and get-ups). Head Pain is a story clip ([RSQ.md](RSQ.md), Motion set).
  Defence stance with a step also answers electricity and energy effects.
- A lethal wound plays Fall Backward and collapses the character for 3 s
  before it dies ([RSQ.md](RSQ.md), The collapse), as Exarust does now.
- It bleeds (REA on the damage routine's blood branch, the wound queue and
  its consumer in the body tick) when its race bleeds (`races.rdb` flag
  bit 0) and the wound's size R = P/(P + 1)·m, before the 0.75 of red,
  exceeds 0.05. The wound waits on the victim, at most seven (a new one is
  dropped when seven wait), with the striking body (the weapon, or the
  hand), the contact point as weights in its tetrahedron, the world point
  and R. Each body tick counts every waiting wound down from 7; the first
  whose count reaches 0 is painted, one a tick, corpses included: 7 ticks
  (84 ms) after the blow (the blow is dealt in the striker's post-step,
  after that tick's body ticks, so the count starts on the next tick).
  The weights give where that point of the
  striker is then; D, how it moved on from the contact point, smears the
  wound, of radius 30·R + 10 cm ([RENDERING.md](../RENDERING.md), Blood).
  A punch bleeds on the struck only: hands never take blood themselves.
- The weapon that deals a bleeding blow draws 0.5·R of blood at once, kept
  within 0 to 1 and never washed off, whether the wound waits or is
  dropped (REA on the same branch: the striking item's blood slot, the add
  routine's 0.5 and clamp); hands have none. It stains the weapon from
  its tip ([RENDERING.md](../RENDERING.md), Blood on weapons).

A guarded blow meets no layer and draws no roll; I below is its impact
at the hit. A weapon's parry deals nothing: the parrying fighter's attack
state clears, it is kept from attacking 600 ms (not with Riposte) and its
stance sags min(I/2, 1) (`damage::parried`). A blow on a shield is the
block path (`damage::block`, REA on its branch, products in double): a
shield parry pays the arm's stamina cost·0.4·I, locks the holder 600 ms
without Counter and sags its stance (0.7 − S/2)·I, at most 1; else under
Synchrony while it counters this attack before 0.3 of its clock
x = clock/0.3 and the arm pays cost·(0.3x + 0.3)·I, otherwise x = 1 and it
pays cost·0.8·I (cost = 1 − block/2, × 2/3 with Endure). The stamina stays
at least 0. With x above 0 the leak Q = (1 − S)·(0.4·I + 0.4·x)·(1 −
block/2) takes 0.05·Q off yellow directly (0.025 under a potion; not the
damage call), at least 0; yellow under 0.05 runs the low-health check.
The daze is (0.3 − 0.2·S)·Q (·I when x is 0) with the stall factor half
the holder's clock with Persist, its clock under Synchrony before 0.3,
else 1. No lasting wound, no shove; either guard makes a striker smaller
than 1.4 recoil. A held shield's stamina refills on the living body's
health tick. An armed bite above 0.1 sticks the weapon
(`bite::stick`): the depth is capped at 1, flags are 0x11000 or 0x211000
around the widened 0.4, strength is 12 times the depth, smoothing adds
0.5, and the count is 3 times the truncated 83·unit·depth. The weapon
layer becomes 0xC0, and the struck part, its parent and its children gain
bit 0x40. The solver pulls that grab while its strain lasts. An unarmed parry answers a swing already in its window and picks the height-table clip. An armed grip uses (1.4 − handling) times the 80-to-250 cm falloff times 0.75 times 0.6, and a shield uses stamina times weight times that falloff times 0.75 times 0.4 with flags 0x800. A blocked striker sets strength and the stall rate to 0.5 and clears the released bit (`swing::recoil`). A class word of 0x90 with a continuation, or with a clock that widens past 0.4 and a counter, stops there. Any other swing sets stalling, cleaving and recoiled, blend 40 and both arm stiffnesses to 0.2, and ends at the widened drive times 0.1 times 400, rounded half to even, plus now plus 200. A follow-through uses that skip and otherwise writes the widened gain times recoil times speed times 0.15, floored at 0 (`swing::follow_through`). A body zone at or under the epsilon sets strength 0. A larger zone uses its square root times the scale. An edge uses one minus the zone times the scale plus the zone. Strength caps at 1. The stall is the widened one minus strength times 0.05, and the end is the scale times 400 rounded half to even. A collapsing body that is not already stepping starts one stagger (`collapse::stagger_step`). A balance that widens above 0.4 scales the reach by the balance, and a lower balance scales the widened reach by 0.4. The span squares the tone plus one half and multiplies by that length and the stored square root of the scale. The target adds the velocity's reciprocal-square-root unit times the span to the chest. A zero velocity leaves the chest. Landings and the step kind are 0. The footwork walks that target (`Ragdoll::walk_stagger`): the stance origin is the target pulled back to 1.1 step lengths of the frame, and one foot is anchored on its shifted spot. A target on the frame moves no foot. The dying tick calls that walk from the measured chest and its fast velocity when no step is running. The same tick drops every held weapon (`collapse::release_hands`): friction 0.75, its own mobility, a drag of that mobility times 0.175, no striker flags, and the velocity it had. The hands are empty afterwards. The weapon becomes its own owner, and each child copies that owner. It leaves its parent's list. The weapon and each descendant take the weapon as their root. Iterations are 2 plus the parts visited, minus 1. Links are 0 and the awake byte is 0. A grip at or above 0 becomes -1. A part with a child, an object type whose low twelve bits are 0x89, and object bit 0x400000, sets its mark to -2. A solid dropped weapon enters the solver (`collapse::free_body`), awake at 256, with its own mobility, friction 0.75, its spheres and its velocity. One solver tick moves it. A flat shape spawns nothing. A dropped weapon whose sphere starts on another body's sphere meets it in the shipped body step (`step_bodies`): one hit, and the weapon leaves the place one step alone would leave it. A lasting wound is noticed (`damage::notices_damage`) when it times 0.75, widened, times 0.2, is greater than 0.25 widened times 0.05. Bits 0x3daaaaab are not noticed. The next float is. The hit path records that notice. A granted second wind fires when the game clock is past its ready clock (`fighter::answer_low_health`), waits 180000 ms, and skips the heartbeat. The heal sound uses volume bits 0x3dcccccd and pitch bits 0x3f800000. At that ready clock the heartbeat message is written. A knock-out (`exa_ai::idle::knock_out`) is the unconscious state. The wake
clock is the clock plus the delay, wrapping. Flags lose bits 0xF0 and gain
the dormant and waking flags. The condition is down, the statue mark
clears, gait is 0, tone is 0, and the pin gain bits are 0x3da3d70a. A
sleep countdown below 256 rises to 256. One already at 300 stays. A blow
does not call it. Health does not store a limb impairment: a part's damage
and stun weights stay the bone table. Not yet: shields as bodies, and
encumbrance's effect on swings.

## Practice ladder

The next fight advances the count by one and stops at 8
(`exa_game::practice::next_practice`). Novice count 1 is `opponent01` and
count 8 is `opponent08`. Expert adds 8, so count 1 is `opponent09` and
count 8 is `opponent16`. Count 8 is the last fight, and a count already
there stays. Red gains the double 0.1 times the maximum and is kept within
it. Yellow copies red. Red bits 0x3e4ccccd with maximum 0.25 become
0x3e666667. Red bits 0x3e75c28f become 0x3e800000. The partner is not
spawned.

## Arena tables

`exa_game::arena` holds the arena's tables as read from `Exanima.exe`
0.9.5's data:

- 6 standard matches;
- 7 daily ranked matches;
- 5 tournament formats;
- 18 weekly tournament prize ids;
- the season's 4 armour sets;
- 19 rival companies: names and three profile bytes;
- 6 loadout tiers, 0x158-byte rows;
- the tiers' point limits.

The record log (`exa_game::arena_log`) grows a list of twenty-byte
records, one id and a value or one id and four words; its two appenders
are a 113-byte routine (an id and a value) and a 127-byte one (an id and
sixteen bytes).

### Arena matches

`exa_game::arena_match` holds the twelve arenas a career match is fought
in, read from the executable's arena tables: `arena1`–`arena7` indoor and
`arenao1`–`arenao5` outdoor, each with the player's spawn spot in
centimetres (the same spots as `exa_world::start`'s arena centres), an
indoor post-process amount (0.3 for arena2, 0.6 for arena4, else 0; what the
post processor does with it is not traced), an outdoor flag that lets the
environment draw take its one-in-six late branch (arenao1–arenao4), and an
outdoor flag that plays the `NtrA` bed by day (arenao1, arenao3, arenao5;
the others play `NtrW` when the wind is over 0.1). The match tracks and
their volumes are in the same module; their names are the sound bank's
(`Sound.rpk` holds `msc_areannov`, the original's spelling). Rules:
`openspec/specs/arena-career`. The hire list is `exa_game::hirelings`, the
arsenal's shelves `exa_game::arsenal`.

`exa_game::match_layout` holds where each template's fighters stand around
the spawn spot: one opponent 2 m ahead (5 cm up) and the company 2 m behind
for the one-on-one matches; a 3.5 m ring at fractions of a turn for the team
matches (Doubles ∓0.05 against 0.55 and 0.45, Valiance ∓1/15, Valour and
Captain −1/12, 0, 1/12, Skirmish and the Beast's company at 7/12, 0.5,
5/12); six places a sixth of a turn apart for a fray or a brawl, each
opponent its own team. A tournament event groups its entry slots into
teams of its team size and stands every member on one ring of
`teams × size × 50 + 100` cm (at most 350), each team spread around its
own sixth (`bracket_layout`): the spread is `f32((n − 1)/(n + 1) · 2)` and
the start `f64(g/G · 6) − f64(f32(spread · 0.5))`, narrowed once.
The hub (`exa_game::arena_hub`) draws Msc_Hub01 or Msc_Hub02 at 0.1 and a
start at none, 41, 126.8 or 152 seconds; its click hook knows the objects
named ArenaShop and ArenaMach by their sixteen-byte names, whose first byte
is 0xAA in the executable's copies.

The staff (`exa_game::staff`) keeps the original's 56-byte employee
records reduced to what its routines read: a class byte (1 a recruit, 2 a
combatant, 4 a merchant, 8 a physician, 0x10 a trainer), the name's sixteen
bytes, a fighter's kit, a specialist's own items, hands and worn items.
Fighter ids are the place on the staff plus one (0x10000000 added for a
recruit, and the manager's word); the tournament entries and the log's
"saved" records refer to them. The manage window's report text is
`exa_game::report`; the rank names are the item card's.

## Actors

`actors.rpk` inside Resource: 21 entries, each a character record (the
layout of [RFC.md](RFC.md#starts-characters-and-placed-objects), versions 6
and 7: 14 and 7) followed by the actor's own item database (`DBDB`):
spawnable character templates with loadouts (`ArenaShpKp`, `opponent01..16`,
`Demon A01`, `proctor`, `villager`, `Derrin`). Every record parses to its
end (`exa-tool characters actors`); the opponents climb from no skills to
opponent16's Close Combat 20000 and Armour 25000 (Manoeuvring tier 3,
Deflection, Security). The sixteen opponents and the proctor store empty
pools (0, either version); ArenaShpKp, the demon, the villager and Derrin
store full yellow and red.

## Roles (`charroles.rdb`)

Status: **verified** (`exa_formats::roles`). All 49 records of the shipped
database parse to their ends; a 50th table entry points outside the file
and is skipped. Established from the role lookup, the add-role walk, the
entry-section finder and the dialogue walkers of `Exanima.exe` 0.9.5 (REA),
then checked against the corpus (`roles::tests::shipped_roles_and_narrator`).

Role record, all little-endian:

| offset | field |
|---|---|
| `+0x00` | u32 record size |
| `+0x04` | u32 flags: bit 0 a relation role (Truce, Ally, Leader…, held toward one other character); bit 16 adding the role adds its parent too; bit 8 (Corlian only) not decoded. The dialogue search and attitude test bit 0; topic activation tests the whole low byte equal to 1 |
| `+0x08` | name[16], NUL-padded |
| `+0x18` | u32 parent role id |
| `+0x1C` | u32 unused (0) |
| `+0x20` | u32 weight in percent (low byte; 0, 50 or 100) |
| `+0x24` | u32 variable count |
| `+0x28` | u32 count of a second 20-byte table (0 in every record) |
| `+0x2C` | u32 0 |
| `+0x30` | u32 entry count |
| `+0x34` | u32 init script length, then the script (no NUL) |
| … | variables: name[16] + u32 type word (0, 1, 2, 4; meaning not decoded) |
| … | entries |

Entry: u32 size (counting itself), u32 kind, u32 target (a role id; 0 the
relation partner; a character id when the condition word has bits 31 and
30 set, the low 30 bits then being the locale), u32 condition, u32 value
(byte 0 attitude −100..100, byte 1 tolerance 0..100). Kind `0x1000` makes
the attitude apply; the other kind bits add sections, stored in this order:

| bit | section |
|---|---|
| `0x1` | u32 length + bytes, never present |
| `0x2` | a dialogue graph (below), sized by its own block size |
| `0x4` | encounter script: u32 length + text |
| `0x8`, `0x10` | u32 length + bytes, never present |
| `0x40` | interaction condition: u32 length + text (`result = …`) |
| `0x20` | corpse script, run when the player handles the holder's corpse: u32 length + text (Corlian only, a `GameEvent(…)` line). It is not in the section table: the original's corpse accessor reads it after the sections of `0x1` to `0x8` only, so a `0x10` or `0x40` section stored with it would read the same bytes |

The first entry whose target and condition words match is the one a lookup
takes; if it has no dialogue the lookup fails, later entries are not tried.

### Dialogue graphs (role entries and `narrator.rcd`)

`narrator.rcd` is one graph on its own (2 actors, `Narrator` and `Player`, 5
nodes: `Corlian`, `XU_Memory`, `RVL_RaiseDead_S`, `RVL_RaiseDead_F`,
`Power_Glean`).

```
u32 target          the aim of the entry (0 in narrator.rcd); memory is filed under it
u32 block size      from the block's start
u32 n actors        then n × { name[16], u32 ARGB colour, u32 0, u32 len + len bytes }
u32 n nodes         then n nodes; an empty node is one zero word
node:   u32 size, u32 flags, title[16], f32 x, f32 y (editor), u32 n links,
        n × u32 link (low 16 bits a node number from 1), then a group
group:  u32 n, n × branch
branch: u32 size, u32 flags, u32 n lines, n × { u32 speaker (actor slot from 1),
        u32 len + text }, u32 len + condition script, u32 len + action script,
        then a group (the replies)
```

Node flags: bits 0–3 re-use cooldown class (0 never again, 1 a year … 13
10 s, 14 0 s, 15 always); bits 4–7 calm class (how low the owner's arousal
must be: 1, 10, 20, 200, 200); bits 8–12 hits that activate a topic, `0x2000`
hits counted as distinct witnesses, `0x4000` no re-activation while active,
bits 16–19 how long an activation lasts (same table as the cooldown), bits
20–22 activation delay (0, 2, 5, 10, 30, 300, 1800 s); `0x1000000` an entry
node, `0x8000000` never an entry; `0x2000000` and bits 28–31 are set in the
data but read by no routine found. Branch flags: bits 0–2 the link to follow
when the branch ends; bits 9–10 the highest hostility it is meant for; bits
4–8 attitude bands (stored, not tested). Text is Windows-1252; `{…}` groups in
lines are script expressions (`{Actor[1].Name}`).

### Topic memory at run time

Each dialogue keeps one u32 per node in the owner's relation record about
the partner, under a relation-role entry and a block key (the stores in
`exa_script::topics`):

- a role-to-role graph: entry = the aimed-at role (the graph's target), key
  = the owner's role;
- a relation role's own graph: entry = the relation role, key = the graph's
  target;
- `ActivateTopicN` aimed at no role: entry 0, key = the role.

The stored block is a u32 node count word followed by one word per node. A
conversation's buffer has the same shape: slot 0 is its own node count and
node `n` is slot `n`. When a dialogue loads a block whose count differs from
its node count, it writes its own count into slot 0, copies the stored
block from the block's count word on into slots 1… (as many words as the
block's count, at most the node count) and zeroes the rest: slot 1
receives the stored count, slot `j` the stored node word `j − 1`, and the
last stored node word is dropped. An equal count copies the block straight. A new
conversation's buffer is its count word and zeros; opening from a stored
block copies the nodes into slots 1… unshifted.
The dialogue search (and a broadcast witness's walk) looks a role-to-role
block up under the role itself, but a block of a relation role toward the
partner under the word the stored role list has at the role's slot,
reading that list as 12 bytes per entry even when a role's variable values
follow it (a quirk kept from the original).

## Material libraries (`.rml`)

Packs of material payloads (the same layout as `BA01` payloads, see
[RFC.md](RFC.md#material-list-ba00)) keyed by material name: `common.rml` (51), `underworld0.rml` (24), `underworldb.rml`,
`humanbase.rml` (`eyehuman`, `skinhuman`). Named in `Exanima.rfp`.

## Shader libraries (`.flb`)

`shading.flb` (54 programs: `ambient`, `bloomfilter`, `blurconv_x/y`,
`dispshell_f/v`, `effectbarrier_f/v`, …) and `textures.flb` (61 procedural
texture programs: `chardecomp_d/n/s`, `chargen_*`, `cloth_*`, …).

The container is the pack table of contents (32-byte records, offsets from
the end of the table). Each entry is rotated word by word by a
position-keyed stream cipher: with a key restarted at `0x2BCBE389` for every
entry, each little-endian u32 advances `key = key·0x10A860C1 mod 2^32`, and
the word is rotated left by `(key >> 13) & 31`, with bit 2 forced when
`(key >> 13) & 7` is zero. The decoded text is the GLSL template the
original's shader builder expands; the language, flag tables, program
choices and uniform blocks are in [FLB.md](FLB.md). `exa_formats::flb`
decodes the library; `exa-tool shader` prints a material's program.

## Manual pages (`.fds`)

`manualkm.fds` (keyboard/mouse) and `manualcn.fds` (controller): packs of
CRLF text with markup tags `[algn=2]`, `[tcol=FFE399]…[/tcol]`. Pages:
armour, combat, encounters, health, interaction, inventory, movement,
saving, …

## Sound

`Sound.rpk`: 1131 RIFF/WAVE PCM 16-bit files. Mono 44.1 kHz (424),
48 kHz (395), 96 kHz (186), 192 kHz (39); stereo 48/44.1/96 kHz (87, mostly
music `msc_*` and ambience `ambn*`). Prefixes: `ambn*` ambience, `door*`,
`foley*`, `stp*` footsteps, `vh[fm]NN_*` voices (grunts, groans),
`thm*` thaumaturgy, `wh_*` weapon hits, `msc_*` music.
