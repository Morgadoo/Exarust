# Saves (`.rsg`, `.rcp`)

Status: **the package, the slot rules and the player-data framing are
verified; the chunk payloads are partly known**. Status words as in
[README.md](README.md).

The game writes saves into the user folder (`%APPDATA%\Exanima`), never
into the installation. A **save** is `ExanimaNNN.rsg` (story) or
`ArenaNNN.rsg` (arena); its **checkpoint** is the same name with `.rcp`.
Both are the same container. A checkpoint is the point a death reloads
from: it is written once per slot and only while none exists, and loading
from one copies it back over the save.

Evidence: the binary's package create, open, read-section, write-section,
delete-section and copy routines; the slot scanner, new-slot, delete,
most-recently-used order, permadeath, legacy-removal and player-data
readers and writers; and the strings they compare. The layouts below were
checked against the instructions, not the pseudo-code, where they differ
(the pseudo-code drops a `− 0x10` from the section-name indexing and
reports a magic test that has no effect). No retail save ships with the
game, so the format is checked by building and reading synthetic packages
here, not against a shipped instance.

## Package

```
u32   magic            0xDA7AEA02 (story and arena alike)
i32   count            entries including the sentinel (at least 1)
entry × count          each 24 bytes:
    u32 offset, u32 size, char name[16]
payloads               at their offsets, normally the next 0x2000 boundary
```

* Entry 0 is the **sentinel**: offset 0, size `0x10` (`8 + 8·count` when
  the file was created with one entry, and never updated), name all
  zeroes. Parsers skip it.
* A section's name is exactly 16 bytes, zero padded (`#playerdata`,
  `#worldobjdb`, `#worldchrdb`, one per visited locale such as
  `exanima04`, and `arenahub` for the arena hub).
* The directory is not sorted. Sections are **not** contiguous: the bytes
  between the aligned end of one and the start of the next are free space,
  and a deleted section's payload stays in the file.
* **Opening** a file shorter than 9 bytes, or one whose first word is not
  the magic, resets it to a fresh package: the magic, count 1 and the
  sentinel, written over the start. The file is not truncated.
* **Writing a section** ([`exa_formats::save_package::PackageImage`]):
  * a section that is not there goes into the first gap that fits it
    (scanning consecutive entries; the room is the next offset minus this
    offset minus the aligned size), else at the end;
  * the **last** section is always rewritten at its offset, even if it
    grows past the old end;
  * any other existing section is rewritten in place when the room up to
    the next section is at least the new size, and otherwise removed and
    reinserted with the gap rule;
  * a new entry's offset is the section before it plus its size, rounded up
    to `0x2000` (the sentinel counts as 0/`0x10`, so the first section is
    at `0x2000`);
  * writing the directory and the payload seeks back and writes; bytes that
    no entry covers are left as they were.
* **Deleting a section** removes its directory entry, shifts the entries
  after it down and rewrites the directory. The payload stays.
* A package's own bytes, as Exarust writes them, are the header, the
  directory and each payload at its offset with zeroes in the gaps.

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
`prefix + 3 + extension` characters long, and the extension exactly four
bytes including its terminator (`.rsg`/`.rcp`).

The number formatter writes `000` for zero, otherwise the decimal digits
left-padded to three. A negative number would come out as `00` in the
original (the padding runs without any digits); Exarust returns `000` for
it, a value the scanner never produces.

The **scanner** walks the folder with `FindNextFile` (so directory order),
skips directories, and for each matching name:

1. keeps only the first file with a given number (the other kind is
   ignored from then on);
2. opens the package (creating or resetting it as above) and reads
   `#worldchrdb` to let the mode find the player;
3. asks the mode's callback; a negative answer **deletes both files** of
   the slot and drops it, a positive one appends the player's name and
   keeps it.

The player name comes from the character record the character-database
importer picks; that importer belongs to the character component. The
count of found slots starts at `−1`, meaning "not scanned yet".

**Ordering.** The list keeps the slots in most-recently-used order. The
order is stored in `Exanima.set` under a key per mode (story
`0x6431AC03`, arena `0x6431AC13`, both observed). After a scan the stored
numbers that are still slots come first, in their stored order, then the
rest in scan order; stored numbers that are gone and repeats are ignored.
The order is written back only when the list has at least two slots.
Loading a slot moves it to the front; creating one puts the new slot at
the front.

**A new slot** takes the lowest number from 1 through the list's length
that no slot uses (numbers above the length do not reserve anything), or
one past the length when all are taken. It is named after the current
player, written, and becomes the current slot.

**Deleting a slot** (by index in the list) closes the package, logs the
index, deletes the `.rsg` and then the `.rcp`, and shifts the rest down.

**Permadeath.** A save is refused while the player's health is below
`0.01` unless the caller forces it. In that case the current slot is
dropped from the list when its checkpoint file does **not** exist (the
check is "the file opens", not its magic). The original's permadeath
routine only edits the in-memory list: it leaves both files on disk, so a
later scan can list the slot again unless something else removes it.

**Checkpoints.** The `CheckpointSave` script function does nothing unless
it is called for the player; when checkpoints are enabled and the `.rcp`
does not exist it saves the `.rsg` and copies it over the `.rcp`. Saving
manually also refreshes the checkpoint this way when there is none.
Loading from a `.rcp` and getting the level up then copies the `.rcp`
back over the `.rsg`.

**Legacy removal** at start-up walks every `<prefix>*<3 bytes>.rsg`, opens
it, reads `#playerdata`, and deletes the `.rsg` when its first word (the
signature) is below the mode's current one. Checkpoints are not touched.

## `#playerdata`

A fixed 48-byte header, then the player's **character placement chunk**
and then tagged chunks. Integers are little-endian; floats are IEEE
single. Both modes share the first 32 bytes.

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
newer save still loads); a lower one makes the slot legacy. The flag byte
at bits 16–23 is the "started" byte (the writer always sets it to 1). When
the start background is 0 and the player record's word at its own offset
0x180 is 0, the reader sets that word to `0x510000`.

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

### Version conversion

* Story: no conversion beyond the signature rule above.
* Arena: the low byte of the data version is compared with 9. Below 9 the
  loader drops the save's `arenahub` section (the hub is regenerated),
  skips the hireables and shop-stock chunks, resets three arena words
  (two to 0, one to 300.0), repairs the player record's item ids and
  compacts the hub stash.
* Story start compatibility (the character creator, while the started byte
  is 0): a save with an `exanima06` section is refused; with an
  `exanima04` section it is refused unless its `#worldobjdb` starts with
  the object-database magic `0xDBAC0D06`.
* The world object database section is a chunk `0xDBDB` holding the object
  database export whose payload starts with an RDB whose magic is the
  version tag above, then a `0xDBCD` chunk with the data manager's table.
  The character database section is one chunk `0xCECDDB00`.
* The settings chunk (`0xECADCA01`) is written into the level section by
  the story's exporter only; see the settings rules in
  [SETTINGS.md](SETTINGS.md) and [`exa_formats::save_chunk`].

### Chunks

Every chunk is `u32 id, u32 size, payload`. The character is a chunk too:
its tag is `0xCAC0CA00` and its payload is the character placement the
level exporter defines ([RFC.md](RFC.md)). The reader takes the chunk
headers while at least eight bytes remain, so a tail of up to seven bytes
after the last chunk is ignored. Unknown ids are skipped by size.

| id | mode | payload |
|---|---|---|
| `0x00000010` | both | journal: `i32 n`, then per entry `i32 id`, string title, string text |
| `0x00000100` | story | discovery: the experience system's own format ([`exa_game::experience`](../../crates/exa_game/src/experience.rs)) |
| `0x00100000` | story | global experience snapshot, u32 |
| `0xD000C001` | story | memory-fragment flags, u32 |
| `0xD000C003` | story | portal/respawn state, 16 bytes; state 2 forces the level to `exanima05` after loading |
| `0x00010000` | arena | hireable characters |
| `0xA0100000` | arena | arsenal, five categories of five item lists |
| `0xA0200000` | arena | hub stash, one item list |
| `0xA0300000` | arena | shop stock: `i32 n`, then `n` ids |
| `0xA0400000` | arena | employees |
| `0xA0500000` | arena | merchant stock and specialisation |
| `0xA0600000` | arena | tournament state |
| `0xA0D10000` | arena | a record with no traced meaning |
| `0xA0DC0000` | arena | company record |
| `0xD0E80001` | Exarust | **guess:** Exarust's session payload; no retail counterpart |

A string is a `u32` byte length and that many bytes (no terminator). A
journal entry's id identifies the dialogue entry that wrote it; Exarust's
journal has no ids and writes the page index. The journal chunk is written
only when it has entries; the memory-flags and transition chunks only when
their word is non-zero; the story always writes the global-experience
chunk.

A loader can pass a **mask**: the reader ands each chunk id with it and
ignores a result of `0xF` or less. The character screen loads a save with
mask `0x100010`, which keeps the journal and the global-experience
snapshot only.

### Section write order

The full save writes: `#worldchrdb` (when the scene has characters),
`#playerdata`, `#worldobjdb` (when it has objects), then the current level
under its locale name (a full level export). Exarust writes only
`#playerdata` until it has a world to export; its `#playerdata` leaves out
the character placement when it has no character, which the original never
does — but writing such a save over one that has a placement keeps the old
placement, as the arena writer does when no scene is loaded.

### Level transitions

* A level is available when the save has a section with the locale's name;
  otherwise the game loads `<locale>.rfc` from the archive.
* Moving to a level the save already holds, while the save also has an
  `exanima04` section (a story in progress), flushes the scene, saves, and
  **copies the save over the checkpoint**, so the checkpoint follows the
  player across levels. This is separate from the once-only checkpoint
  write.
* When the saved transition state (`0xD000C003`) is 2, the story loader
  forces the locale to `exanima05`; the load path then places the player at
  the portal node and sets the state to 1.

## Practice

Practice has no save file of its own in this unit: the practice and arena
combatants are prepared by rebuilding their kit from the object database
(the prep helpers), and the mode's saved state is the arena's. A practice
fight started from the arena therefore writes `ArenaNNN.rsg`.

## Exarust mapping

[`exa_formats::save_package`](../../crates/exa_formats/src/save_package.rs)
holds the container, the slot file names and the player data;
[`exa_formats::save_chunk`](../../crates/exa_formats/src/save_chunk.rs)
holds the settings chunk, the save number format and the slot, death,
checkpoint and clock rules; [`exa_game::session`](../../crates/exa_game/src/session.rs)
writes and reads a session in this shape, and
[`exa_game::slots`](../../crates/exa_game/src/slots.rs) does the folder
work.
