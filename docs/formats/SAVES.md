# Saves (`.rsg`, `.rcp`)

Status: **the package, the slot rules and the player-data framing are
verified; the chunk payloads are partly known**. Status words as in
[README.md](README.md).

The game writes saves into the user folder (`%APPDATA%\Exanima`), never
into the installation. A **save** is `ExanimaNNN.rsg` (story) or
`ArenaNNN.rsg` (arena); its **checkpoint** is the same name with `.rcp`.
Both are the same container. A checkpoint is the point a dead player comes
back from: dying deletes the save, and the next load then opens the
checkpoint and copies it back over the save.

Evidence: the binary's package create, open, read-section, write-section,
delete-section, copy and delete routines; the slot scanner, new-slot,
delete, most-recently-used order, permadeath, legacy-removal, load, save,
checkpoint and transition routines, the two modes' player-data readers and
writers and the character-placement reader; and the strings they compare.
Arithmetic (the gap scan, the clock, the death test) was read from the
instructions, not the pseudo-code, where they differ. No save ships with
the game; the layout was also checked against two saves the retail game
wrote on this machine (one story, one arena, read in place and not copied
anywhere): both rebuild byte for byte from their directory and payloads,
and their player data matches the layout below field for field. The
repository's tests use synthetic packages of the same shape; a test reads
a folder of real saves when `EXANIMA_SAVES` names one.

## Package

```
u32   magic            0xDA7AEA02 (story and arena alike)
i32   count            entries including the sentinel (at least 1)
entry × count          each 24 bytes:
    u32 offset, u32 size, char name[16]
payloads               at their offsets, each at the 0x2000 boundary after
                       the entry before it
```

* Entry 0 is the **sentinel**: offset 0, size `0x10` (`8 + 8·count` when
  the file was created with one entry, and never updated), name all
  zeroes. Parsers skip it; its size still counts for placing the entry
  after it, so the first section is at `0x2000`.
* A section's name is exactly 16 bytes, zero padded, compared as 16 raw
  bytes (`#playerdata`, `#worldobjdb`, `#worldchrdb`, one per visited
  locale such as `exanima04`, and `arenahub` for the arena hub).
* The directory is not sorted. Sections are **not** contiguous: the bytes
  between the aligned end of one and the start of the next are free space,
  and a deleted section's payload stays in the file.
* **Opening** a file opens it for reading and writing, creating it empty
  when it is missing. A file of 8 bytes or fewer, or one whose first word
  is not the magic, is given the magic and **reset** in memory: count 1
  and the sentinel (offset 0, size 16). The open writes nothing (the reset
  runs before the file's handle is kept, and only writes through a kept
  handle); the next section write puts the header and the directory over
  the file's start and leaves the rest, so a damaged file the slot scan
  opens stays as it is. A file with the magic keeps
  its directory count as stored; a count of zero or a negative count (the
  word is signed) reads as an empty package, as the original's reader
  simply loops over no entries.
* **Writing a section** ([`exa_formats::save_package::PackageImage`]):
  * the **gap scan** walks every pair of consecutive entries, the sentinel
    included; a pair's room is the next offset minus this offset minus this
    size rounded up to `0x2000`, in wrapping 32-bit arithmetic. The scan
    keeps the **last** pair whose room is **strictly larger** than the new
    size and places the section after that pair's first entry; with no such
    pair the section is appended;
  * a section that is not there is inserted where the gap scan says;
  * an existing section that is not the last one, and whose room up to the
    next entry (that offset minus its own) is below the new size, is moved
    to where the gap scan says (the scan runs with the section still in
    place), the entries between shifting over it; any other existing
    section stays at its place in the directory;
  * in every case the written entry's offset is then recomputed as the
    entry before it plus its size, rounded up to `0x2000`. A section
    rewritten "in place" after its predecessor shrank therefore moves down
    to the predecessor's new aligned end;
  * the directory and then the payload are written over the file; bytes no
    entry covers are left as they were, and the file never shrinks.
* **Deleting a section** removes its directory entry, shifts the entries
  after it down and rewrites the directory. The payload stays.
* **Probing** a file opens it for reading and discards its first eight
  header bytes; success means the file opened, whatever its size or magic.
* **Deleting a package** closes it, resets it in memory and deletes the
  file.
* Copying a package to its checkpoint (or back) is a plain file copy. The
  checkpoint name is the save's extension stripped and `.rcp` appended; the
  save name is the checkpoint's extension stripped and `.rsg` appended.
* A package's own bytes, when Exarust writes one, are the header and
  directory written over the file as it was read, then each payload at its
  offset. Bytes no entry covers — a gap, a deleted section's payload, an
  old tail — stay as they were, and the file never shrinks. Both retail
  saves observed have zeroed gaps and end at their last payload. A package
  built from nothing has zeroes in its gaps.

## Slot files

| file | meaning |
|---|---|
| `<prefix><NNN>.rsg` | the save |
| `<prefix><NNN>.rcp` | its checkpoint |

`<prefix>` is `Exanima` for the story and `Arena` for the arena. `NNN` is
exactly three characters, and the scanner reads them **arithmetically**:
`(b0 − '0')·100 + (b1 − '0')·10 + (b2 − '0')` with no digit check, so
`1:2` reads as 202. The number must be 1 through 999. Prefix and
extension are compared ignoring ASCII case; the file name must be exactly
`prefix + 3 + 4` characters long (`.rsg`/`.rcp`).

The number formatter writes `000` for zero, otherwise the decimal digits
left-padded to three. A negative number comes out as `00` in the original
(its digit loop does not run, so only the two-character pad is added);
Exarust reproduces that, though the scanner never produces such a number.
The name builder copies at most the first fifteen bytes of the prefix.

The **scanner** walks the folder with `FindNextFile` in directory order
(the very first entry the search returns, normally `.`, is skipped), skips
directories, and for each matching name whose number is not already listed:

1. opens the package (creating or resetting it as above) and imports
   `#worldchrdb`;
2. runs the mode's player-data reader on it with load mask 0 (the header
   and the character, no chunks); the arena reader still converts an old
   save's header first, so the scan deletes its `arenahub` section;
3. a negative answer **deletes both files** of the slot; a positive one
   lists the slot with the player's name; zero leaves the slot out and its
   files alone.

The **first file for a number decides that number's fate**, even when the
answer is zero: the number is put in the list before the reader runs, so a
later `.rcp` of the same number is skipped. A negative answer deletes both
files and removes the number again.

The **story** reader answers negative only for a `#playerdata` whose
signature is below `0x6F5AEA14`, and reads any higher signature, the
arena's included, as a story. The **arena** reader takes its signature
exactly and answers zero otherwise (with the message "Save game format no
longer supported."), so an arena scan never deletes. Either answers zero
when the section is missing or the character's record is not found. The
player's name comes from that record: the placement's reference bit 30
selects the world character database, the low 28 bits its record id, and
the record's name field is what the list shows
([`exa_game::session::player_name`](../../crates/exa_game/src/session.rs)).
The count of found slots starts at `−1`, meaning "not scanned yet";
Exarust rescans on every list.

**Ordering.** The list keeps the slots in most-recently-used order. The
order is stored in `Exanima.set` under a one-tag path per mode (story
`0x6431AC03`, arena `0x6431AC13`; the store call takes a path whose first
word is its length, 1). After a scan of at least two slots the stored
numbers that are still slots come first, in their stored order, then the
rest in scan order; stored numbers that are gone and repeats are ignored.
The order is written back only when the list has at least two slots.
Loading a slot moves it to the front (a number not in the list changes
nothing); creating one puts the new slot at the front.

**A new slot** takes the lowest number from 1 through the list's length
that no slot uses (numbers above the length do not reserve anything), or
one past the length when all are taken. It is named after the current
player, its package is created empty, and it becomes the current slot.

**Deleting a slot** (by index in the list) closes the package, logs the
index, deletes the `.rsg` and then the `.rcp`, and shifts the rest down.

**Loading a slot** moves it to the front of the order, then opens the
first of its files that **exists** (opens for reading): the `.rsg` and
then the `.rcp`, or the `.rcp` first when the caller asks for the
checkpoint. A file that exists
but does not read is a failed load; the other file is not tried. The full
load reads `#worldobjdb`, `#worldchrdb`, the player data with every chunk,
and the locale's section (or `<locale>.rfc` from the game archive). An old
arena save is converted first: its `arenahub` section is deleted from the
package and the package is written back. The story loader forces the
locale to `exanima05` when the transition state is 2. Once that succeeded
from a `.rcp`, the checkpoint is copied over the `.rsg`, and the package
switches back to the `.rsg` name. A load that fails while reading a `.rcp`
also switches back, creating an empty `.rsg` when it is missing but not
copying the checkpoint into it. The character screen's load prefers the
`.rsg`; it takes the `.rcp` only when asked or when the `.rsg` is missing,
and reads the player data with mask `0x100010` (the journal and the global
experience only). It does not move the slot in the order.

**Saving.** A save is skipped while a "saving disabled" flag is set; the
flag is checked before the health. A save that checks the player (the
death and arena paths) does not write when the player is dead and runs the
permadeath rule instead; the menu, checkpoint and transition saves do not
check. A save writes, in order: `#worldchrdb` (when the world character
database has records), the mode's `#playerdata`, `#worldobjdb` (when the
object database has records), then the current level under its locale name
(a full level export; the story's exporter adds the settings chunk).
Sections already in the package that are not written this time are left
alone; nothing is deleted.

**Permadeath.** The player is dead when the health, widened to double, is
below the double `0.01` (so the single-precision `0.01`, which is
`0.0099999998`, is dead; a NaN health is not). Death **deletes the slot's
`.rsg`**; when no `.rcp` exists (the test is only that the file opens) the
slot is also dropped from the list and the current slot is cleared. With a
checkpoint the slot stays listed from its `.rcp`, which the next load opens
and copies back.

**Checkpoints.** The `CheckpointSave` script function does nothing unless
it is called for the player; when checkpoints are enabled and no `.rcp`
exists it saves the `.rsg` and copies it over the `.rcp`. Saving from the
menu, and the transition save below, copy the save over the checkpoint
**unconditionally**, so those paths refresh it.

**Legacy removal** at start-up walks every `<prefix>NNN.rsg`, opens it,
reads `#playerdata`, and deletes the `.rsg` when its first word (the
signature) is below the mode's current one (story `0x6F5AEA14`, arena
`0xE8AAEA10`, compared unsigned). Checkpoints are not touched.

## `#playerdata`

A fixed 52-byte header, then the player's **character placement chunk**
and then tagged chunks. Integers are little-endian; floats are IEEE
single. Both modes share the first 36 bytes.

### Story header

| offset | type | meaning |
|---:|---|---|
| 0 | u32 | signature `0x6F5AEA14` |
| 4 | char[16] | locale (the level name) |
| 20 | f32[3] | camera target (cm) |
| 32 | f32 | camera zoom |
| 36 | u32 | save seed, drawn when the character was created |
| 40 | u32 | flags: low byte the start background; bit 16 is set by the writer |
| 44 | u32 | game clock in seconds |
| 48 | u32 | reserved, written zero |

The story reader accepts **any signature at or above** `0x6F5AEA14` (a
newer save still loads); a lower one makes the slot legacy. It reads the
camera target and zoom into scratch and does not restore them, and it
skips the reserved word. The flag byte at bits 16–23 is the "started" byte
(the writer always sets it to 1). When the start background is 0 and the
player record's own flag word is 0, the reader sets that word to `0x510000`. Before the chunks it resets the journal to one "New
Entry" page, the global experience to the default worth (zero), the memory
flags and the transition state to zero, so an absent chunk means the
default. Exarust exposes those defaults through `journal_or_default`,
`global_experience`, `memory_flags` and `transition_state`. The first chunk
after the header is the character whatever its id; Exarust's own saves omit
the placement, so its reader only takes a chunk tagged `0xCAC0CA00` as the
character (`parse_retail` follows the original's rule).

### Arena header

| offset | type | meaning |
|---:|---|---|
| 0 | u32 | signature `0xE8AAEA10` |
| 4 | char[16] | locale (`arenahub`) |
| 20 | f32[3] | camera target (cm) |
| 32 | f32 | camera zoom |
| 36 | u32 | money (the company's copper) |
| 40 | u32 | arena data version (current 9) |
| 44 | u32 | game clock in seconds |
| 48 | u32 | day counter |

The arena reader takes its signature **exactly**; anything else shows
"Save game format no longer supported." and the slot is not loaded. A new
arena's money word is `515` (the value the writer's global starts at);
Exarust stores the company's copper there and also in its own chunk.

### Character placement

The first chunk after the header. The writers tag it `0xCAC0CA00`; the
reader takes the first chunk as the character **whatever its id**. When no
scene is loaded, the arena writer copies the previous save's `0xCAC0CA00`
chunk instead of exporting one.

| offset | type | meaning |
|---:|---|---|
| 0 | u32 | character reference: bit 30 set looks the record up in the world character database (the one `#worldchrdb` restores), clear in the other character database; the low 28 bits are the record id |
| 4 | f32[3] | position (cm): body x, floor height, body z |
| 16 | f32 | heading (radians) |
| 20 | i32 | condition (zero for the living player in both observed saves) |
| 24 | chunks | sub-chunks to the chunk's end, kept by the loader for the scene import: flat pose `0xCAC0E100`, motion state `0xCAC0EA00`, pose tree `0xCAC0EB00`, attached objects `0xCAC0EC00`, AI home `0xCACE0F77` (both observed saves hold the flat pose and the motion state) |

This is the level exporter's single-placement layout ([RFC.md](RFC.md),
`exa_formats::rfc_write::write_character_state`); the reader's field order
matches it.

A reference the lookup does not find makes the player-data reader answer
zero: the slot is neither listed nor loaded.

### Version conversion

* Story: no conversion beyond the signature rule above.
* Arena: the low byte of the data version is compared with 9. Below 9 the
  loader drops the save's `arenahub` section (the hub is regenerated),
  puts the player's saved position back to `(0, 0, 300)`, skips the
  hireables and shop-stock chunks, regenerates the hireables when the
  mask keeps them, repairs the player record's item ids and compacts the
  hub stash. Exarust's load does the section drop, the position, the
  chunk removal and the stash compaction, and writes the converted player
  data back so a later save does not revive the skipped chunks (the
  original keeps the empty lists in memory only). It carries the pure
  compaction passes (`arena_save::retain_plain_items`,
  `arena_save::retain_plain_records`, `remove_duplicate_item_ids` and the
  record-repair order in `ArenaLists::compact`) and the player record's
  repair (`arena_save::repair_converted_record`: the worn list, each
  loadout's worn list, then the carried items, before the record's
  re-derive; the stash is compacted afterwards by its own pass). The
  repair is a tested function: Exarust's load does not decode and
  re-encode the world character database to apply it. Regeneration and
  the object-database id passes are not modelled.
* Story start compatibility (the character creator, while the started byte
  is 0): a save with an `exanima06` section is refused; with an
  `exanima04` section it is refused unless its `#worldobjdb`'s **third
  word** (the first word of the object export's payload) is the
  object-database magic `0xDBAC0D06`; a missing `#worldobjdb` passes.
* The world object database section is a chunk `0xDBDB` holding the object
  database export (an RDB whose magic is the tag above), then a `0xDBCD`
  chunk with the data manager's table; both chunks are always written. The
  reader takes the first and second chunk whatever their ids. The
  character database section is one chunk `0xCECDDB00` whose payload is an
  RDB with magic `0xDBACCD00` (observed in both retail saves).
* The settings chunk (`0xECADCA01`) is written into the level section by
  the story's exporter only; see the settings rules in
  [SETTINGS.md](SETTINGS.md) and [`exa_formats::save_chunk`]. It is written
  only when the read tag was old or the saved word is above zero (unsigned).
  It holds the twelve-byte tag (the tag read from the save when it was old,
  otherwise the current one: a zero, `1 100 000.0`, a zero) and the two
  words the settings store carries. `rfc_write::export_save_level` appends it after
  the scene's chunk tree.

### Chunks

Every chunk is `u32 id, u32 size, payload`. The reader takes chunk headers
while **more than** eight bytes remain, so a tail of up to eight bytes
(an empty chunk at the very end included) is ignored. Unknown ids are
skipped by size.

| id | mode | payload |
|---|---|---|
| `0x00000010` | both | journal: `i32 n`, then per entry `i32 id`, string title, string text |
| `0x00000100` | story | discovery: the experience system's own format ([`exa_game::experience`](../../crates/exa_game/src/experience.rs)) |
| `0x00100000` | story | global experience **worth**, u32 (zero by default; the writer always emits it) |
| `0xD000C001` | story | memory-fragment flags, u32 |
| `0xD000C003` | story | portal/respawn state, 16 bytes; state 2 forces the level to `exanima05` after loading |
| `0x00010000` | arena | hireable characters |
| `0xA0100000` | arena | arsenal: 25 item lists, one per shelf and box, written shelf by shelf |
| `0xA0200000` | arena | hub stash: one item list |
| `0xA0300000` | arena | shop stock: `i32 n`, then `n` ids |
| `0xA0400000` | arena | employees |
| `0xA0500000` | arena | merchant stock and specialisation |
| `0xA0600000` | arena | tournament state |
| `0xA0D10000` | arena | a record with no traced meaning |
| `0xA0DC0000` | arena | company record |
| `0xD0E80001` | Exarust | **guess:** Exarust's session payload; no retail counterpart |

A string is a `u32` byte length and that many bytes (no terminator). A
journal entry's id identifies the dialogue entry that wrote it; Exarust's
journal has no ids and writes the page index.

**Write order.** The story writes the placement, discovery (always), the
journal (when it has entries), the global experience (always), the memory
flags and the transition state (each when non-zero). The arena writes the
placement, the journal (when it has entries), the arsenal (always), the
stash and the shop stock (each when not empty), then the company record
`0xA0DC0000`, the extra record `0xA0D10000`, the employees `0xA0400000`
(when not empty), the merchant `0xA0500000` (skipped only when the staff
is non-empty and holds no merchant, class 4), the hireables `0x00010000`
and the tournament `0xA0600000`; the observed arena save holds, after the
placement, `0xA0100000`, `0xA0300000`, `0xA0DC0000`, `0xA0D10000`,
`0xA0500000`, `0x00010000`, `0xA0600000` (no employees yet).

The arena's employees chunk is a count followed by one record per
employee: two unused words, the profession word, a custom-fighter flag
(a flagged record then has no character id), the character id otherwise,
an item list (a count and twelve-byte item references) and four words. The
merchant chunk is the stock type, the specialisation word (kept raw; it
starts at −1), two unused words and the stock's count and ids; a loader
that regenerates the stock reads only the header. The hireables chunk
(`0x00010000`) is a count and, per candidate, an unused word and a
character export. An **item list** is a signed count and that many
twelve-byte entries (id, flags, a third word). The arsenal is 25 such
lists: the writer walks the five shelves and the five boxes inside each
shelf, so the wire order is shelf 0 box 0…4, shelf 1 box 0…4, and so on
(the shelf indices are the arsenal's: weapons 0, shields 2, armour 3,
clothing 4). The stash is one list.

A loader passes a **mask**: the reader ands each chunk id with it, ignores
a result of `0xF` or less and dispatches the rest on the **masked** id
(with a narrow mask an unrelated id can land on a known one). The full
load passes all bits, the slot scan 0, and the character screen
`0x100010`, which keeps the journal and the global-experience snapshot
only.

### Level transitions

* A level is available when the save has a section with the locale's name;
  otherwise the game loads `<locale>.rfc` from the archive. Exarust exposes
  the same order as `Slots::level_available`, with the archive's lookup
  supplied by the caller.
* The **transition save** runs when the save has **neither** an
  `exanima04` section **nor** a section for the target level (the early
  levels, entered for the first time): it flushes the scene, saves without
  the health check and **copies the save over the checkpoint**. A save with
  either section is left alone.
* When the saved transition state (`0xD000C003`) is 2, the story loader
  forces the locale to `exanima05`; the load path then places the player at
  the portal node and sets the state to 1.
* **Placement.** The player's matrix is the saved placement; its height is
  probed down 300 cm from 100 cm up, then lifted by `108.18 − 11.1·extent`
  and 5 cm (13 cm for a body in its raised state). The body's feet are put
  at the probed floor and the body is carried to the lifted point.
* **Followers.** When the current scene character is flushed, every
  travelling character (controller word 4, flags without 0x800) joins the
  migration list once; a repeat is logged as `Duplicate simulation motile:
  <name>` under `Migration` and skipped. Followers take the six stored
  offsets `(−100,0,−100)`, `(100,0,−100)`, `(0,0,150)`, `(100,0,200)`,
  `(0,0,0)`, `(0,0,0)`, each taken as a row vector times the spawn
  matrix's rotation rows (the saved player matrix, else the identity),
  added to the player's placed position, and then spawned with its own
  floor probe and lift. The original does not bound the index (a seventh
  follower reads past the table); Exarust stops at six. A plain scene
  flush empties the migration list after flushing its characters.
  Re-attaching one frees its UID manager, re-creates its player controls,
  clears its carry word, stores 1 as its body controller and raises a minimum distance under 5 cm to 5 (a NaN is
  kept, as the unordered compare does not branch).
### Loading a level

Three entry points open a level; all store the current locale's sixteen-byte
name on the scene once the import succeeds.

* **Save load.** When the load asks for it and the save has a section named
  by the bare locale, that section is imported, with the save's own local
  database as the importer's context; otherwise the archive's
  `<locale>.rfc` (the locale with its extension replaced) is read. A file
  that cannot be found returns without importing. The previous level id is
  stored as 0, the portal node as −1 (none) and the current level id comes
  from `locales.rdb`.
* **Named level** (`exanima01`, `arena`, `arenae`, `arenahub`): the archive
  is opened first; then the save manager is ensured, the local character
  database is cleared, the GUI is reset and the load's globals are reset
  (the settings flag, tag and saved word, and the scene's sun height).
  This path first clears the save-disable flag. After a successful import
  the scene environment's word that the outdoor preset sets to 1 (and the
  indoor preset to 0) is cleared; it is not the sun mode, which goes
  through its own setter.
* **Arena level**: the archive only; no GUI or settings reset. Only the item
  GUI's panel reset runs first (the save load runs the same reset first).
* All three clear the portal node (a null pointer: the level's own start),
  store −1 in the engine's portal id word and reset the saved spawn matrix
  to its default.

**Import order.** Reserve the kd-tree build; import the chunk tree; free the
section stream; release the reservation; when at least one node came back:
begin the scene-content pass, prepare the kd-tree and texture passes,
classify the nodes into the scene's lists (see above), fix the node count,
reset the renderer's GI manager when it has one, load the terrain (below; a
terrain entry found and read counts as loaded), tell the arena task
when the terrain is missing, run the tree and texture passes again, add the
nodes to the scene, rebuild the sector and bounds passes, run the final node
pass, update the grid environments when the scene has a grid, and report
the scene ready. A malformed section imports nothing and still frees its
stream; a section with no nodes does the same. A section imported from the
save also reads the level's settings chunk (the tag with its version float
and the two words) as it goes; the archive's own level has none.

**Game scene.** The scene is created with its flag word `0x10000`, 115 sound
slots [guess at the field's meaning], node-manager flags 1 and 1, the scene
environment's sun mode 1 (off, through the presets' own setter) and fog
distance `0x3F333333` (0.7), the view's own
byte set and its setter called with 1, and the renderer's option word 3;
the object manager, audio environment, editor, GI manager, blood, AI hooks
and the save unit's scene-content classifier are created on the way. The
indoor preset (which sets the sun) is applied by the caller just after.

**GUI reset.** Every load, combatant preparation and the menu scene runs
the same reset: the GUI's task list, the tips/hover state (with argument
1), the shared game functions, the item GUI, then a global word cleared.

**Player spawn.** The player's body comes from the default generator (the
character record) or, when a custom generator is installed, from repeated
calls with a zero sleep until it answers. The attached nodes' part pass
runs, the root node gains `0x1000`, the player behaviour, inventory and
controls are attached, and the node is added to the scene before the
generator's finishing pass. The player node's own part controller is put in
mode 8, the node is made visible, two body words are cleared and the node is
registered.

**Menu.** The menu background is the shipped `menuscene.rfc` with the
1.8 fog preset (the indoor preset variant), the scene editor at mode 0, the
view at (0, 500, 700) and its target at (0, 0, −200); a 60 is written into
the camera (most likely its field of view). The arena task is reset when it
exists, the portal word goes to −1, the GUI and the dialogue engine are
reset, the open form is closed and the game scene becomes the current
scene, and the import uses the file's own name for its terrain probe
(`menuscene.rft`). A word of the scene's grid goes to 0, the editor to mode 0 and the
camera's matrices are updated last.

**Terrain.** A saved level's terrain file is the locale with its
extension replaced by `.rft` (the loader looks it up in the archive and
imports it when it exists).

**Scene content.** A scene node is classified from its object record:
kind pair `0x1100`/`0x1200` goes to the first list, `0x2200` to the
second, and independently a `0x10000` property whose first byte is 2
adds it to the third (one node can join two lists). A record the
object database does not know falls back to a name starting with `[`,
which joins the second list.

**Load reset.** Before a load the save unit clears the old-settings flag,
leaves the default twelve-byte settings tag (a zero, `1 100 000.0`, a
zero), clears the settings chunk's saved word and the scene's sun height,
and allocates the grid environment's arrays (a zero word per grid cell,
update flag 1 and mode 0).

**Landing.** The landing plan puts the player at the saved planar
position and the lifted floor height, then places the followers around
them from the offset table in order (positions before each follower's
own floor probe).

**World databases.** The load imports the save's `#worldchrdb` first chunk
into the world character database (a missing section clears the live one)
and the `#worldobjdb` first chunk into the object database's world tier,
with its second chunk as the string library the records read their text
through (a missing section clears the live object database). Exarust
exposes both as `Slots`' `world_characters`/`world_objects`
(`exa_game::slots`) and on a `LoadedSave`.

**Flow.** A transition closes the open forms, reserves the kd trees, waits
13 ms, resets the audio environment and the GUI, begins the object
database's save pass, puts the player's body part controller in mode 8,
flushes the player and collects the travelling characters, ends the object
pass, sets up each travelling character's node, activates the player's
node, restores the GUI's audio state and resets the scene content, leaves
the arena task, looks up the destination's locale and level ids, clears the
portal node, stores the **previous** level's id in the portal word (the
other entry points store −1 there), resets the load's globals, opens the save, reads the
destination's section from the save or the archive, imports it, stores the
name, chooses the spawn matrix (the portal node's when one is named, else
the identity), spawns the player, stores its transform, re-derives a
story-mode-2 character and clears the word to 1, sets up the grid arrays
when the flag is set, spawns the followers on the stored offsets,
runs the follower re-attach pass (always, even with no followers) and the
game functions' post-load pass. A destination that is neither in the save
nor the archive aborts before the import and leaves saves enabled; an
archive destination that reads but does not import disables saving, while
a section read from the save counts as imported whatever its import
returned and the landing goes on.

## The game clock

The clock is the seconds read from the save plus the whole seconds since
the load: the millisecond tick counter and the tick at load are both
zero-extended to 64 bits, subtracted, and the **signed** difference divided
by 1000, rounding toward zero; the sum is kept to 32 bits. A tick before
the load tick therefore counts back (−2001 ms is −2 s) instead of jumping.

Exarust keeps the pair in [`exa_formats::save_chunk::GameClock`]; the
player-data reader's setter is `exa_game::world_clock::restored` and the
writer's read is `exa_game::world_clock::stamped`, with
`Session::world_clock` and `Session::stamp_world_clock` applying them to a
save. The running game restores the base when it receives a
`RestoreWorldClock` message: the host sends a loaded save's seconds, and a
session start sends zero so the world time begins at its load rather than at
the process's start (the elixir's 1800 s stamp reads it).

## Practice

Practice has no save file of its own in this unit: the practice and arena
combatants are prepared by rebuilding their kit from the object database
(the prep helpers), and the mode's saved state is the arena's. A practice
fight started from the arena therefore writes `ArenaNNN.rsg`.

A fighter's saved kit names its items through the character's **own**
database (an id whose top two bits are `10`). The preparation pass resolves
the first hand pair, the second pair, the worn items and then the carried
items; a carried item that names a hand's saved id takes that hand's
resolved id instead of being resolved itself, and the original stores the
*second* pair's replacements from the *first* pair (a dead value that still
decides that lookup). The equip pass then instantiates every own id into the
running scene in the same order, writes the first pair's new ids into the
attached scene nodes and the character's hand node that still hold the old
ones, gives carried items the running hand id when they name one (first pair
first), frees the temporary database and refreshes the item GUI.

The combatant scene is prepared in this order: the open form is closed,
the game scene becomes the current scene, the GUI state helper the
transition also calls runs, the portal word goes to −1, the kd trees are reserved, the GUI and
the content helpers reset, then the archive's `<locale>.rfc` is opened
and imported with the saved locale for its terrain probe. A successful
import lets the arena task run its post-load hook when its flag bit 0 is
clear.

## Exarust mapping

[`exa_formats::save_package`](../../crates/exa_formats/src/save_package.rs)
holds the container, the slot file names, the player data (including the
writer's chunk order and the reader's defaults), the character placement
and the world database sections;
[`exa_formats::save_chunk`](../../crates/exa_formats/src/save_chunk.rs)
holds the settings chunk, the save number format and the slot, death,
checkpoint and clock rules;
[`exa_formats::arena_save`](../../crates/exa_formats/src/arena_save.rs)
holds the arena table payloads and the load's list compactions, and
[`exa_game::arena_tables`](../../crates/exa_game/src/arena_tables.rs) fills
them from and applies them to the live company, staff, arsenal and
tournament roster (the league's own chunks read back on a load);
[`exa_game::session`](../../crates/exa_game/src/session.rs)
writes and reads a session in this shape (the mode, level, journal, used
topics, company, roster, focus, clock, seed, start background, discovery,
global experience worth, memory flags and transition state), and
[`exa_game::slots`](../../crates/exa_game/src/slots.rs) does the folder
work: open, scan, load (`load_package`, with the arena conversion and the
forced transition locale), the character screen's load
(`load_character_screen`), save (`save_full` in the writer's section
order), checked save, checkpoint (`checkpoint_full`), death, delete,
legacy removal and transition save (`transition_save_full`).
[`exa_game::save_scene`](../../crates/exa_game/src/save_scene.rs) holds the
three scene-load entry points (`scene_load_plan` and `load_level`), the
import order, the game scene's constants, the menu scene's load, the GUI
reset and the player's node set-up and spawn;
[`exa_game::save_kit`](../../crates/exa_game/src/save_kit.rs) the practice
and arena fighters' two kit passes and their scene preparation, and
[`exa_game::level_transition`](../../crates/exa_game/src/level_transition.rs)
the transition's ordered flow over those pieces.
`create_empty` is the original's new-slot path (an empty package);
`create` also writes a session into it.

**Compatibility.** A package Exarust writes is laid out as the original's
writer lays it out, and a retail save read and written back is unchanged.
Exarust's `#playerdata` carries the mode's header, the journal, the
story's always-written discovery and global-experience chunks (an old
save's chunks are kept and reordered, never lost) and its own chunk. It
has no world to export yet, so a save Exarust writes **from nothing** has
no character placement: the original's reader would take its first chunk
(the empty discovery chunk) as the character, fail the record lookup, and
leave the slot out of its list (without deleting it). A session saved
**over** a retail save of the same mode keeps the save's placement, its
other sections and the chunks Exarust does not produce, so the original
still loads it; the Exarust chunk is an id it skips. Exarust lists its own
saves without the record lookup, and shows the player's name when the
placed record is in the save's `#worldchrdb`.
