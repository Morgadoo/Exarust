# save-games Specification

## Purpose
The save game package: the settings chunk, its tags and versions, and the easing curves the save screens use.

## Requirements

### Requirement: Save easing curve
Easing SHALL return `(x × 2)² × 0.5` below 0.5 and `((2x − 3)(2x − 1) − 1) × −0.5` otherwise.

#### Scenario: The curve
- **WHEN** the input is 0.25
- **THEN** the result is 0.125
- **WHEN** the input is 0.5
- **THEN** the result is 0.5
- **WHEN** the input is 0.75
- **THEN** the result is 0.875

### Requirement: Save settings chunk
Writing SHALL use the header `0xECADCA01`, a small or large tag and two dwords. Reading SHALL accept only that header, set the old flag when 1,000,000 is above the tag float, and store the dwords only on acceptance.

#### Scenario: Write and read
- **WHEN** the large tag is written with 3 and 4
- **THEN** the header is `0xECADCA01`, the tag is large and the dwords are 3 and 4
- **WHEN** the header matches and the tag float is 999,999
- **THEN** the read is accepted, the old flag is set and both dwords are stored
- **WHEN** the tag float is 1,000,000
- **THEN** the old flag is clear
- **WHEN** the header differs
- **THEN** the read is rejected and no dword is stored

### Requirement: Save version gate
Conversion SHALL be false unless the mode is 4 and the pending count is zero. A legacy entry SHALL convert. Otherwise a present and read record SHALL convert when its tag is not `0xDBAC0D06`, and the same tag SHALL not.

#### Scenario: Legacy, current and refused
- **WHEN** the mode is 4, pending is 0 and the legacy entry is present
- **THEN** conversion is needed
- **WHEN** the record is read with tag `0xDBAC0D06`
- **THEN** conversion is not needed
- **WHEN** the tag is 0
- **THEN** conversion is needed
- **WHEN** the mode is 3 or pending is positive
- **THEN** conversion is not needed

### Requirement: Save record table
Writing SHALL write the count first, then each record's kind and two qwords.

#### Scenario: Two records
- **WHEN** two records are written
- **THEN** the count is 2
- **AND** each record keeps its kind, first qword and second qword in order

### Requirement: Save number format
Formatting SHALL give `"000"` for zero or a negative value. Otherwise it SHALL write the decimal digits and pad on the left with zeros to three characters, leaving longer values whole.

#### Scenario: Zero, short and long
- **WHEN** the value is 0
- **THEN** the text is 000
- **WHEN** the value is 5
- **THEN** the text is 005
- **WHEN** the value is 12
- **THEN** the text is 012
- **WHEN** the value is 1234
- **THEN** the text is 1234
- **WHEN** the value is −7
- **THEN** the text is 000

### Requirement: Save package
The system SHALL read and write the retail package: the magic, the sentinel and 24-byte directory, the 16-byte section names and the 0x2000 alignment.

#### Scenario: Two sections
- **WHEN** a package is written with `arenahub` and `exanima04`
- **THEN** the entries start at 0x2000 and 0x4000 and the total is 0x4009
- **WHEN** the package is parsed
- **THEN** both sections are returned by their exact names

### Requirement: Save chunks
The system SHALL frame payloads as `u32 id, u32 len, payload`.

#### Scenario: One chunk
- **WHEN** chunk 0x100000 with four bytes is written and parsed
- **THEN** its id and payload round-trip

### Requirement: Player-data headers
The system SHALL read and write the 52-byte story (`0x6F5AEA14`) and arena (`0xE8AAEA10`) headers: the signature, the 16-byte locale, the camera target and zoom, then four mode words (story: seed, flags, clock, a zero word; arena: money, data version, clock, day). The story reader SHALL accept any signature at or above its own; the arena reader SHALL accept only its own.

#### Scenario: Story and arena
- **WHEN** a story or arena header is written with no character
- **THEN** it is 52 bytes, and a character chunk written after it starts at byte 52
- **WHEN** a story header names exanima04 with flags 0x10003
- **THEN** the flavour is Story, the level is exanima04, the start background is 3 and the started byte is 1
- **WHEN** the story reader meets signature 0x6F5AEA15
- **THEN** it reads the header
- **WHEN** it meets 0x6F5AEA13, or the arena reader meets the story signature
- **THEN** the read is refused

### Requirement: Package gap scan
A new section SHALL go after the first entry of the last pair of consecutive entries, the sentinel included, whose room (next offset minus this offset minus this size rounded up to 0x2000, wrapping) is strictly larger than its size, or at the end when no pair has such room.

#### Scenario: Gaps
- **WHEN** sections sit at 0x4000 and 0x6000 and a 0x100-byte section is added
- **THEN** it goes at 0x2000, after the sentinel
- **WHEN** sections sit at 0x2000, 0x6000 and 0xA000 (0x10 bytes each) and a 0x100-byte section is added
- **THEN** it goes after the second, at 0x8000
- **WHEN** the added section is 0x2000 bytes instead
- **THEN** it does not fit either room and is appended at 0xC000

### Requirement: Package rewrites
An existing section that is not the last one and whose room up to the next entry is below its new size SHALL move to where the gap scan puts it. Every written entry's offset SHALL be recomputed from the entry before it.

#### Scenario: Rewrites
- **WHEN** a 0x3000-byte first section at 0x2000 shrinks to 0x10 bytes and the section after it, at 0x6000, is rewritten
- **THEN** the rewritten section moves to 0x4000
- **WHEN** a retail-shaped story save is read and written back, or written from scratch section by section
- **THEN** both results equal the original bytes

### Requirement: Package opening
Opening a package file SHALL create it empty when it is missing and SHALL read a file of eight bytes or fewer, or one with another magic, as a fresh package (the magic, count 1, the sentinel of offset 0 and size 16) without writing anything; the next section write SHALL put the header and directory over the file's start and leave the bytes after them.

#### Scenario: A damaged slot file
- **WHEN** a story scan meets a 7-byte file named Exanima007.rsg
- **THEN** the file keeps its 7 bytes, is not listed and is not deleted

#### Scenario: A missing file
- **WHEN** a new slot is created
- **THEN** its save file exists and is empty

#### Scenario: A later write
- **WHEN** a section is written into a 100-byte file with another magic
- **THEN** its first 56 bytes are the header and two directory entries and bytes 56 to 99 are as they were

### Requirement: Character placement
The first chunk after the header SHALL be read as the player's placement: the record reference (bit 30 selects the world character database, the low 28 bits are the id), the position, the heading, the condition and sub-chunks. An old arena save SHALL put the position back to (0, 0, 300).

#### Scenario: Placement
- **WHEN** a placement with reference 0x40000001, position (8183.75, −6, 3961), heading −1.5, condition 0, and the flat-pose and motion sub-chunks is read
- **THEN** the record id is 1, it is in the world database, and every field and sub-chunk round-trips
- **WHEN** the level exporter writes a dead character's placement and it is stored as the player data's character
- **THEN** it reads back with the exporter's reference, position (floor height as y), heading and condition
- **WHEN** an arena save of data version 0x108 is converted
- **THEN** the position becomes (0, 0, 300), the hireables and shop-stock chunks are dropped and the other sub-chunks stay

### Requirement: Player-data chunks
After the character, chunks SHALL be read while more than eight bytes remain. A load mask SHALL be anded with each id, results at or below 0xF dropped and the rest kept under the masked id.

#### Scenario: Tail and masks
- **WHEN** an empty chunk is the last eight bytes
- **THEN** it is not read, and one more byte makes it read
- **WHEN** chunks 0x110 and 0xA0100000 are masked with 0x100010
- **THEN** they are kept as 0x10 and 0x100000
- **WHEN** discovery, journal, experience, memory and transition chunks are masked with 0x100010
- **THEN** the journal and experience remain

### Requirement: World database sections
The object database section SHALL hold the object export chunk 0xDBDB and the data manager chunk 0xDBCD, both always written. The story start check SHALL refuse a save with `exanima06`, and one with `exanima04` unless the object section's third word is 0xDBAC0D06; a missing object section SHALL pass.

#### Scenario: Start check
- **WHEN** the object section is the export chunk of the magic
- **THEN** the save can be started from
- **WHEN** the magic is the section's first word, or the export holds 0xDBAC0D05
- **THEN** it cannot

### Requirement: Slot scan
A scan SHALL take slot files in directory order, list a number once, open each file, and decide per mode: the story deletes both files of a slot whose signature is below its own and lists any other readable header; the arena lists only its exact signature and deletes nothing; anything else is left out and on disk.

#### Scenario: Verdicts
- **WHEN** a story folder holds a valid slot, a garbage file, an old-signature slot with a checkpoint, a package without player data and an arena header under a story name
- **THEN** the valid slot and the arena-headed one are listed, both files of the old slot are deleted, and the rest stay
- **WHEN** an arena folder holds a story header and an old story signature under arena names
- **THEN** neither is listed nor deleted
- **WHEN** a slot has only its checkpoint, then both files
- **THEN** it is listed once, from the checkpoint the first time

### Requirement: Slot order and numbers
The list SHALL keep the stored most-recently-used order first (stored numbers still listed, in order, repeats ignored), then the rest in scan order, and SHALL store it only for two or more slots. A load SHALL move a listed slot to the front. A new slot SHALL take the lowest free number from 1 through the list's length, or one past it, at the front.

#### Scenario: Order
- **WHEN** slots 3, 1, 2 are found and 2 is stored
- **THEN** the order is 2, 3, 1
- **WHEN** slots 1, 2, 3 are created, 1 is deleted, a slot is created and 2 is loaded
- **THEN** the new slot is 1 and the order is 2, 1, 3, also in a fresh folder handle

### Requirement: Slot load
A load SHALL open the first of the slot's files that exists, the checkpoint first when asked; a file that exists but does not read SHALL fail the load without trying the other. A load from the checkpoint SHALL copy it over the save.

#### Scenario: First existing file
- **WHEN** the save exists but is not a session and the checkpoint is
- **THEN** a plain load fails and a checkpoint load succeeds
- **WHEN** only the checkpoint exists
- **THEN** a plain load reads it and the save file exists again

### Requirement: Permadeath
A player whose health, widened to double, is below 0.01 SHALL be dead. Death SHALL delete the slot's save and SHALL drop the slot when no checkpoint exists. A checked save of a dead player SHALL write nothing and apply death; an unchecked save SHALL write at any health.

#### Scenario: Death
- **WHEN** the health is 0.010001
- **THEN** nothing happens
- **WHEN** the health is the single 0.01 and there is no checkpoint
- **THEN** the save is deleted and the slot is no longer listed
- **WHEN** a checkpoint exists and the health is 0
- **THEN** the save is deleted, the slot is listed from its checkpoint and the next load restores the checkpoint's clock
- **WHEN** the health is NaN
- **THEN** the save is allowed

### Requirement: Checkpoints and transitions
A checkpoint SHALL be written only when none exists, by saving and copying the save; a manual save SHALL refresh it unconditionally. The transition save SHALL run only when the save has neither an `exanima04` section nor one for the target level, and SHALL refresh the checkpoint.

#### Scenario: Transition save
- **WHEN** a fresh story save moves to exanima02
- **THEN** it saves and the checkpoint holds the new clock
- **WHEN** the save already holds exanima02
- **THEN** a move there writes nothing, and a move to exanima03 saves
- **WHEN** the save holds exanima04
- **THEN** a move to exanima05 writes nothing

### Requirement: Legacy removal
At start-up every slot save whose player-data signature is below the mode's current one SHALL be deleted; checkpoints SHALL stay.

#### Scenario: Old story save
- **WHEN** a story folder holds a current save and one with signature 0x6F5AEA13 plus its checkpoint
- **THEN** one save is removed, the current one and the old checkpoint stay

### Requirement: Game clock
The clock SHALL be the saved seconds plus the signed millisecond difference since the load divided by 1000, truncated toward zero, kept to 32 bits.

#### Scenario: Clock
- **WHEN** 120 s were restored at tick 5000
- **THEN** ticks 5999, 6000, 65000, 4500, 3000 and 2999 give 120, 121, 180, 120, 118 and 118
- **WHEN** 0 s were restored at tick 1000 and the tick is 0
- **THEN** the clock is 4294967295
- **WHEN** 120 s were restored at tick 4294966296 and the tick is 1000
- **THEN** the clock is 4290672451

### Requirement: Story writer order
After the placement, the story writer SHALL emit the discovery chunk, the journal when it holds an entry, the global experience, the memory flags when non-zero and the transition state when its first word is non-zero. Chunks the writer does not know SHALL follow, in the order read.

#### Scenario: Story order
- **WHEN** transition, unknown, experience, journal, memory and discovery chunks are set in that order
- **THEN** they are written as discovery, journal, experience, memory, transition, unknown

### Requirement: Arena writer order
After the placement, the arena writer SHALL emit the journal when it holds an entry, the arsenal, the stash when non-empty, the shop stock when non-empty, the company record, the extra record, the employees, the merchant, the hireables and the tournament. Chunks the writer does not know SHALL follow, in the order read.

#### Scenario: Arena order
- **WHEN** the tournament, hireables, company, shop stock, arsenal, journal, employees, extra, merchant and stash chunks are set in that order
- **THEN** they are written as journal, arsenal, stash, shop stock, company, extra, employees, merchant, hireables, tournament

### Requirement: Arena merchant and arsenal chunks
The arsenal chunk SHALL be 25 item lists written shelf by shelf with the five boxes inside. The merchant chunk SHALL be skipped only when the staff is non-empty and holds no merchant (class 4).

#### Scenario: Merchant condition
- **WHEN** the table writer runs with no staff, with a staff of classes 2 and 8, and with a staff holding class 4
- **THEN** the merchant chunk is written, skipped, and written

### Requirement: Save section order
A save SHALL put the world characters when their count is above zero, the player data, the world objects when their count is above zero, and then the level under its locale name. A player whose health, widened to double, is below 0.01 SHALL have the save deleted and nothing written.

#### Scenario: Sections
- **WHEN** a story save is made with world characters, world objects and a level
- **THEN** the directory lists them in that order after the sentinel

### Requirement: Save inspection
A save package SHALL be readable without being written. Reading and rewriting an untouched package SHALL give the original bytes, a deleted section's payload and the bytes between sections included.

#### Scenario: Rebuild
- **WHEN** a package is read and written back with no change
- **THEN** the bytes are equal
- **WHEN** its player data is parsed and written back
- **THEN** the section is equal

### Requirement: Outdoor scene preset
The outdoor preset SHALL draw from a local generator started at its seed (a zero seed replaced by a fresh one): a day time of 0.32 + 0.42 times the eased draw unless nights are allowed and a six-way roll is zero, then a night time of 0.76 + 0.24 times the draw; it SHALL rain on three of seven rolls.

#### Scenario: Dry day and rainy night
- **WHEN** the preset runs with seed 7 and no nights, and with seed 1 and nights allowed
- **THEN** the first is a dry day at time 0.3605303 with cloudiness 1.251776, fog 0.50763315 and wind 0.53202665; the second is a rainy night at 0.9666516 with rain 0.34562913 and fog 0.3662673

### Requirement: Outdoor preset weather streams
The rain target SHALL be 0.1 + 0.9 times a draw, raised by the rain setter's 0.3 floor before the fog reads it; the setter's thunder draw SHALL come from the engine's shared generator, never the preset's local one; the fog, cloud and wind sums SHALL be computed in double where the original computes them.

#### Scenario: Thunder from the shared stream
- **WHEN** seed 6 draws a rain target of 0.91921914 at time 1000
- **THEN** the next strike is the shared generator's first draw below 30 000 plus 5000, and the local fog, cloud and wind are 0.3195439, 2.3890958 and 0.46616265

### Requirement: Indoor scene preset
The indoor preset SHALL turn the sun light off and the sky off through their setters, set a 50 000 far plane, fog distance 2 (or the variant's 1.8, 1.4 or 2.0), the grey 0.75 global light, a black background, mid-grey fog, no wind and no rain.

#### Scenario: Indoor fog
- **WHEN** the indoor preset is applied
- **THEN** the fog span is 200 100 and the sun mode is off

### Requirement: Level transition placement
A follower SHALL be placed at the player's placed position plus its stored offset taken as a row vector times the spawn matrix's rotation rows, summed first row, second, then third; the offsets SHALL be (−100, 0, −100), (100, 0, −100), (0, 0, 150), (100, 0, 200) and two zeros. A re-attached follower SHALL get controller word 1, a cleared carry word and a minimum distance raised to 5 below 5 (a NaN kept).

#### Scenario: Quarter turn
- **WHEN** the first offset is placed with rows (0, 0, −1), (0, 1, 0), (1, 0, 0) around (10, 0, 20)
- **THEN** the follower lands at (−90, 0, 120)

### Requirement: Scene content lists
A scene node with an object record SHALL join the first list for kind pair 0x1100 or 0x1200, else the second for 0x2200, and independently the third when its 0x10000 property's first byte is 2. A node without a record SHALL join the second list only when its name starts with `[`.

#### Scenario: Two lists
- **WHEN** a node's record has kind pair 0x1100 and a 0x10000 byte of 2
- **THEN** it joins the first and the third list

### Requirement: Scene character flush
Flushing a scene character SHALL register it when its reference has the top bit set, then remap its worn items, its first hand pair, its attached scene nodes holding a first-pair saved id, its second hand pair and its carried items in that order; a carried item holding a hand's old id SHALL take that hand's new id, the first pair looked up first. Loadouts SHALL be left alone. A repeated migration address SHALL be reported and not added to the travelling list.

#### Scenario: Hand reuse
- **WHEN** hands 40, 41, 42, 52, worn 45 and carried 50, 41, 52 are flushed with a remap adding 100
- **THEN** the remap sees 45, 40, 41, 42, 52, 50 and the carried items become 150, 141, 152

#### Scenario: Attached nodes
- **WHEN** attached nodes 40, 41, 99 are flushed with that remap
- **THEN** they become 140, 141, 99

### Requirement: Settings chunk on a saved level
The story's level exporter SHALL write the settings chunk only for an old read tag or a saved word above zero (unsigned), carrying the read tag when it was old and the current tag (0, 1 100 000, 0) otherwise.

#### Scenario: Gate
- **WHEN** the tag is current and the saved word is 0
- **THEN** no settings chunk is written

### Requirement: Outdoor preset view value
The outdoor preset SHALL hand the view zero on a day; on a night with the view flag set it SHALL hand `0.5·(cloudiness − 2)` in single precision plus `(previous time of day − 0.7)` in double, narrowed, both read before the preset's own weather pass.

#### Scenario: Night value
- **WHEN** the preset draws a night from seed 1 with cloudiness 1.5 and time of day 0.7516 standing
- **THEN** the view value is −0.19839998

### Requirement: Old arena record repair
Converting an old arena save SHALL compact the player's record in this order: the worn list, each loadout's worn list, then the carried items, each dropping entries whose id has the top bit set; the hands SHALL be left alone; the record SHALL then be re-derived. The hub stash SHALL be compacted afterwards by its own pass.

#### Scenario: Compaction
- **WHEN** worn 1, 0x80000002, 3, loadouts (1, 0xC0000002) and (0x80000003, 4, 0x80000005) and carried 0x80000001, 6 are repaired
- **THEN** worn is 1, 3, the loadouts hold 1 and 4, and carried holds 6

### Requirement: Level transition flow
A transition SHALL abort before importing when the destination is neither in the save nor the archive. A destination read from the save SHALL land whatever its import returns; an archive destination whose import fails SHALL set the save-disable flag and stop. A landing SHALL always run the follower re-attach pass and the post-load pass. The portal word SHALL take the previous level's id.

#### Scenario: Save section lands
- **WHEN** the destination is read from the save and its import reports nothing
- **THEN** the player is spawned and saves stay enabled

### Requirement: Fighter kit passes
The preparation pass SHALL resolve own ids (top bits `10`) in the first hand pair, the second pair, the worn items and the carried items; a carried item naming a saved hand id SHALL take the stored replacement, the second pair's replacements being the first pair's ids. The equip pass SHALL instantiate in the same order, rewrite every attached node and the hand node holding a first-pair saved id, and give carried items the running id of the matching hand.

#### Scenario: Second pair replacement
- **WHEN** a carried item names the second pair's saved id
- **THEN** the preparation gives it the first pair's id in that position

### Requirement: Level import
The import SHALL reserve the trees, import the chunk tree, free the section and release the reservation; with nodes it SHALL classify the content, reset the GI manager, probe `<name>.rft`, tell the arena task when that entry is missing (an entry found and read counts as loaded), complete the passes, register the nodes, rebuild the scene, update grid environments when the scene has a grid and report ready.

#### Scenario: Unparsed terrain
- **WHEN** a terrain entry is read but does not parse in an arena
- **THEN** the arena task is not told the terrain is missing

### Requirement: World database export
A save SHALL write `#worldchrdb` only when the world character database holds a record, as one chunk `0xCECDDB00` with the database flushed and then saved (magic `0xDBACCD00`, table size `(records + 1) × 16`, table and payload); it SHALL write `#worldobjdb` only when the object world tier holds a record, as chunk `0xDBDB` with the tier's writer followed by chunk `0xDBCD` with the writer of the world tier's own string library.

#### Scenario: One character
- **WHEN** the world character database holds one live record
- **THEN** the export begins with `0xDBACCD00` and a table size of 32 and the save's import reads the record back

#### Scenario: Empty databases
- **WHEN** both databases are empty
- **THEN** neither section is exported

### Requirement: Player placement export
The player-data writers SHALL export the player's placement in save mode under the player's record remembered at the first save, whatever record the body holds, and SHALL flush the record first unless the transition state's first word is 2; the placement SHALL be the first chunk after the header.

#### Scenario: Pending respawn
- **WHEN** a later save runs with the transition word 2 and the body holding another record
- **THEN** the placement carries the first record and the flush is skipped

### Requirement: Arena list regeneration
After an arena load's id pass each employee whose profession byte is 4 SHALL have its own list emptied; every other employee and then the manager SHALL have each loadout's first two non-zero hand ids and its worn ids other than 8 added by a unique append of `(id, 0, 0x28 | 4 << 16)`, the special duplicates removed and each entry shelved with flag `0x400` when its shelf holds no copy; every arsenal box SHALL then be deduplicated.

#### Scenario: Worn id eight
- **WHEN** the manager's loadout wears ids 8, 12 and 11 and holds 11
- **THEN** 12 and 11 are listed once each and 8 is not

### Requirement: Live transition flow
Each in-process level change SHALL record the transition flow with no save package open: an archive destination that exists SHALL read the archive and import, a non-zero spawn SHALL choose the portal matrix, and a destination missing from the archive SHALL abort after the save is opened.

#### Scenario: Missing level
- **WHEN** a change names a level the archive lacks
- **THEN** the recorded flow is aborted and saves stay enabled

### Requirement: Arena item classification
The arena load's id pass SHALL classify an item from its object record's sections: an unknown id SHALL go on no shelf; a body section SHALL put it on the armour shelf when the body's armour type byte is 3 to 6 and on the clothing shelf otherwise; without a body a weapon section (`0x4000`) SHALL put it on the weapons shelf, else a shield section (`0x8000`) on the shields shelf, else on no shelf. Only the sections' presence SHALL count.

#### Scenario: Body wins
- **WHEN** a record has a body with armour type 6 and a weapon section
- **THEN** the item goes on the armour shelf

#### Scenario: Short weapon section
- **WHEN** a record without a body has a four-byte weapon section that does not parse
- **THEN** the item goes on the weapons shelf

### Requirement: Header camera pair
Both writers SHALL store the view camera's place (the translation of the viewport camera node's matrix, the eye, in centimetres) and the camera controller's current zoom word in the header after the locale. The running game SHALL keep the pair on the chase camera's eye and shown zoom each frame; without a camera a save SHALL write the origin and zoom 1. Both readers SHALL read the pair and drop it: a read session SHALL keep the origin and zoom 1.

#### Scenario: Write only
- **WHEN** a story session with camera (123.5, −45, 678.5) and zoom 1.75 is saved and read back
- **THEN** the header holds those values and the read session has the origin and zoom 1

#### Scenario: A live save takes the chase camera
- **WHEN** a save runs while the chase camera's eye stands at (69.8125, 15.75, 32.5) m with its starting zoom
- **THEN** the header holds the place (6981.25, 1575, 3250) and the zoom 1300

### Requirement: Save disable in the live saves
While the last level change has saving disabled, a new-slot save and a flow save step SHALL write nothing and SHALL not apply death. A checkpoint step SHALL skip only the save: when no checkpoint exists the existing save SHALL still be copied over it.

#### Scenario: Stale checkpoint
- **WHEN** saving is disabled, the slot's save holds old bytes and no checkpoint exists, and a checkpoint step runs
- **THEN** the save is unchanged and the checkpoint holds the same old bytes

### Requirement: Checkpoint step order
A checkpoint step SHALL, only when the slot has no checkpoint, write the save without the death check and then copy it over the checkpoint; an existing checkpoint SHALL be left unchanged.

#### Scenario: Existing checkpoint
- **WHEN** a checkpoint already exists and a checkpoint step runs
- **THEN** its bytes do not change

### Requirement: Retail save restore
A story or arena save without an Exarust payload SHALL restore the reader's globals: the clock, the seed, the start background, the journal chunk or the single "New Entry" page, the discovery, global experience, memory and transition chunks, and the level from the header's locale or `exanima05` when the transition state is 2; an arena save SHALL restore the clock, copper, day and the tables it carries.

#### Scenario: Forced locale
- **WHEN** a retail story section names `exanima02` and its transition state's first word is 2
- **THEN** the restored session's level is `exanima05`

### Requirement: Converted arena record on load
The live arena load SHALL apply the old arena record repair to the player's record when the save was converted from an older data version, before the identity reset.

#### Scenario: Converted load
- **WHEN** a converted save's player record wears 1, 0x80000002, 3 and carries 0x80000001, 6
- **THEN** the restored record wears 1, 3 and carries 6

### Requirement: Player health stored by a story save
Before a story save writes the player's record, the system SHALL store the player body's health into it: yellow and red as value / maximum × 65535 and focus × 65535, in single precision rounded half to even (red 0 unless positive), the seed 0xFFFF, a heal-pool entry when the heal pool is positive, then a potion entry with the expiry while the potion flag is set.

#### Scenario: Wounded human
- **WHEN** a human (maximum 0.25) with yellow 0.1, red 0.18, focus 0.4 and heal pool 0.03 is saved
- **THEN** the record holds pools 26214, 47185 and 26214, seed 0xFFFF and one heal-pool entry of 0.03

#### Scenario: Drunk potion
- **WHEN** a human with yellow 0.0625, red 0.15, focus 0 and a potion until 5400 s is saved
- **THEN** the pools are 16384, 39321 and 0 and the only entry is the potion's, 5400

### Requirement: Player health loaded at spawn
A player spawned from a record SHALL load its health: red then yellow as maximum × (stored / 65535) in double precision, focus as stored / 65535, and the heal pool and the potion (flag and expiry) from the saved entries, every known entry taking the last entry's value.

#### Scenario: Wounded human loaded
- **WHEN** the wounded human's save is loaded
- **THEN** the player has yellow 0.1, red 0.17999923, focus 0.4 and heal pool 0.03, without the potion flag

#### Scenario: Potion loaded
- **WHEN** the potion save is loaded
- **THEN** the player has yellow 0.06250095, red 0.15, the potion flag and expiry 5400

#### Scenario: Heal pool and potion together
- **WHEN** the saved record holds a heal-pool entry and then a potion entry with expiry 5400
- **THEN** the loaded heal pool is the bits of 5400 read as a float and the potion expiry is 5400

### Requirement: Checkpoint script
A script's `CheckpointSave` call made for the player while the settings' checkpoints are on SHALL run the checkpoint step on the current story slot (the save, then its copy over the checkpoint when the slot has none); a call for another character, or with checkpoints off, SHALL write nothing.

#### Scenario: Load inside the checkpoint zone
- **WHEN** a story slot without a checkpoint is loaded with the player standing in `exanima01`'s checkpoint zone
- **THEN** the save is written and then copied, and the checkpoint's bytes equal the save's

#### Scenario: Another character
- **WHEN** the call reports a character that is not the player
- **THEN** neither file changes

### Requirement: Loaded character request
After a story or arena load clears the player record's race byte, the load SHALL make the content stream's request for the loaded character with the request word `0x1000` (no AI start, no saved AI pose restored).

#### Scenario: Story load
- **WHEN** a story slot with a player record is restored
- **THEN** the restore carries a request with word `0x1000`

### Requirement: World library import
A load SHALL read the `#worldobjdb` section's second chunk into the world tier's own string library, not the shared one.

#### Scenario: The world library
- **WHEN** the world tier holds a record and its library the text "rusty key"
- **THEN** the save's import has that text in the world tier's library and the shared library stays empty

### Requirement: Live world sync
A story save, the checkpoint's save and a level change SHALL first promote the player's local items to the world tier with the link remap table on and then rewrite the local doors' links; the player's possessions SHALL take the new ids, and the travelling followers' carried items SHALL be promoted the same way. A story or arena load SHALL import the save's world tier into the live database; a new story SHALL clear it.

#### Scenario: A repaired item through a save
- **WHEN** a kit copy of a marked template in the pack has its dirt and wear lowered, the story saves and the save is loaded
- **THEN** the pack names the copy's world id and the live world record keeps the lowered dirt and wear

### Requirement: Arena save promotion
An arena save SHALL, with the id remap table on, promote the manager's record (loadouts included), then every stash id, every entry of the arsenal's five shelves of five boxes and each employee's record and list, then rewrite the scene's local entity ids through the table, and turn the table off.

#### Scenario: The arena's stores
- **WHEN** the stash names local 1, an arsenal box local 2 and an employee's hand, pack and list locals 5, 4, 3 and 1, and the arena saves
- **THEN** they become world 1, world 2, world 3, world 4, world 5 and world 1

### Requirement: Saved level section
Every save SHALL write the current level's section under its locale name after the world databases, holding the live local tier and its library and the placed objects as the scene holds them: a spawned object at its live placement with its live id and condition, a spawned object that is gone left out with the objects inside it, an unspawned one as loaded, and an object dropped since added at the end.

#### Scenario: A taken and a dropped object
- **WHEN** a spawned object holding another is gone and an item was dropped into the world, and the game saves
- **THEN** the section's list leaves both out, closes up the later container numbers and ends with the dropped item

### Requirement: Saved scene characters
A story save SHALL write the scene's characters into the level section: each stored placement whose character stands in the scene takes its live place, heading and death when it moved or died, one whose record left the level's character database is left out, one with no character is kept, and a scene character with no placement is added with its record's id; the section's character database SHALL be the live local one.

#### Scenario: A carried follower saved and loaded
- **WHEN** Derrin follows the player from exanima01 to exanima02, the story is saved there and the save is loaded
- **THEN** the save's exanima02 section places 0x40000002 beside the level's own characters and the load places Derrin from the world character database

### Requirement: Level from the save's section
A story load SHALL read the level from the save's own section when the package holds one, and SHALL not fill that level's containers.

#### Scenario: A dropped repaired item
- **WHEN** `exanima01`'s own trousers are repaired in the pack, dropped into the world, the story saves and the save is loaded
- **THEN** the level is read from the save's section with one more placed object and the trousers' record is the repaired one

### Requirement: Practice kit carry
The next practice fight SHALL prepare the player's own items into the kit's temporary database before the arena's level goes and equip them into the new level's local tier after it loads; that level change SHALL not promote the player's items.

#### Scenario: The carry around the load
- **WHEN** the next practice fight's steps are built
- **THEN** the preparation comes before the level load and the equip after the practice filter

### Requirement: Loaded player placement
Loading a story slot SHALL keep its save file, and SHALL place the loaded player's root and every part of its body at the saved place, in the running simulation as well as in the drawn world.

#### Scenario: A reloaded save
- **WHEN** a story save made at (−4.46, 0, 69.52) m in exanima06 is continued
- **THEN** the player stands there, does not fall, and the save file is still in the folder

### Requirement: Legacy pass at start-up
When the running game is given a save folder, making the slot managers SHALL run the legacy removal over the story manager's saves and then the arena manager's, before any slot is listed; a pass that fails SHALL be logged and SHALL not stop the start-up.

#### Scenario: An old story save in the folder
- **WHEN** the game starts with a save folder holding a current story save and one whose player-data signature is 0x6F5AEA13
- **THEN** the old save is deleted before the menu lists the slots and the current one stays

### Requirement: Followers travel through a door
A level change to a named level that no world reset accompanies SHALL take every scene character whose applied activity is the follow (4) along, each once, before the old level is torn down: its record with its live possessions and health, its local items promoted to the world tier, and its held roles. A change behind a world reset, or a teardown with no destination, SHALL carry nobody.

#### Scenario: A save load
- **WHEN** a save is loaded while a companion follows
- **THEN** no follower is carried into the loaded level

### Requirement: Followers land around the player
Once the destination's collision exists each travelling follower SHALL be placed on the stored follower offsets around the player's landing (turned by the placement's heading, else the identity), on its own floor probe, and respawned with the roles it held and its carried health.

#### Scenario: A recruited companion takes a door with the player
- **WHEN** Derrin follows the player in exanima01 and the player changes to exanima02 through a door
- **THEN** Derrin is respawned in exanima02 at the first offset, (−100, 0, −100) cm from the player with no portal matrix, and still follows

### Requirement: Live world character database
The running game SHALL keep a world character database: a new story SHALL hold the player's slot under id 1, a story load SHALL import the save's `#worldchrdb`, and a story save SHALL write the store's records beside the player.

#### Scenario: A save loaded back
- **WHEN** a story save whose `#worldchrdb` holds the player and a follower is loaded
- **THEN** the live store holds both records

### Requirement: Followers registered in the world character database
A follower taking a door SHALL be registered in the world character database: a record that is not the world store's SHALL keep its origin (the level it leaves and its local id) and join the store under its first free id with the world tag, and every registered record SHALL take the level it leaves as its level. On arrival its record SHALL take the destination's id as its level and keep its world id.

#### Scenario: Derrin registered at a door
- **WHEN** Derrin (record 0x2b) follows the player from exanima01 (level 2) to exanima02 (level 3) in a new story and the game is saved there
- **THEN** his id becomes 0x40000002 with origin (2, 0x8000002b), and the save's `#worldchrdb` holds the player under 0x40000001 and Derrin under 0x40000002 with level 3

### Requirement: Live local character database
The running game SHALL keep the level's own character database: it SHALL be read from the level's section at each load, a follower registered at a door SHALL be deleted from it, and a story save SHALL store each scene character's live record into it before writing it.

#### Scenario: Derrin leaves exanima01
- **WHEN** Derrin (record 0x2b) is registered in the world character database at a door
- **THEN** record 0x2b is no longer in the level's character database

### Requirement: Scene characters flushed by a story save
Before writing its sections a story save SHALL store every scene character's live possessions and health into its record, in the world character database for a world record and in the level's for the rest, and SHALL put the live player's record over the world database's player slot.

#### Scenario: A save after a door
- **WHEN** the story is saved in exanima02 with Derrin following
- **THEN** the world character database holds the live player under id 1 and Derrin's live record under 0x40000002

### Requirement: Saved body of a dead character
A story save SHALL write the live pose tree of every scene character who died since the load: each body part's world frame, its bone number and its part children, depth first from the pelvis. A body without parts SHALL write one record at its feet, so the section still exports.

#### Scenario: A killed NPC saved and loaded
- **WHEN** an exanima01 NPC dies, the story is saved and the save is loaded
- **THEN** the level comes from the save's section and the NPC lies dead on its saved 18-record pose where it fell

### Requirement: Saved motion state
A story save SHALL write, for each living scene character an AI drives, its live pose tree and its motion state (the current motion's number and progress). A load SHALL start the saved motion with no blend and put its progress back; a saved attack motion (5 to 12) SHALL load as the defence stance (3) with no progress.

#### Scenario: Idle NPCs saved and loaded
- **WHEN** the story is saved in exanima02 and the save is loaded
- **THEN** the level's NPCs resume motion 0x13 at their saved progress

### Requirement: Saved AI home
A story save SHALL write the AI home (position and heading) of each living guardian whose race word pair is `0x1000`, whose statue byte is clear and which holds a home. A load SHALL set the guardian's home and heading from it and clear its statue byte.

#### Scenario: An awake guardian's home
- **WHEN** an awake golem with a home is saved and the save is loaded
- **THEN** the loaded golem holds the saved home and heading and is no statue

### Requirement: Saved roles
A story save SHALL store each scene character's live roles into its version 7 record, so a role taken while the level ran is kept by a load.

#### Scenario: A follower saved and loaded
- **WHEN** Derrin follows the player to exanima02, the story is saved and the save is loaded
- **THEN** Derrin loads with roles 31, 19 and 1 and follows (activity 4)

### Requirement: Scene content lists at every load
Each level load SHALL classify its class-1 scene nodes and placed objects as the save unit's node hook does: volumes of kind pair `0x1100` or `0x1200` in the first list, bracket-named nodes without a record and volumes of pair `0x2200` in the second, operative objects of class 2 in the third.

#### Scenario: exanima01 loads
- **WHEN** exanima01 loads
- **THEN** its nine door-side volumes are listed

### Requirement: Reset to portal question
After each select of the saves' character screen that reads a slot, and when the screen opens, the story SHALL raise the `Reset to portal` box when the save read has the third byte of its player data flags at 0 and holds an `exanima06` section, or holds `exanima04` with a world object database magic other than 0xDBAC0D06. The arena SHALL never raise it.

#### Scenario: A pre-0.9.5 save past the portal
- **WHEN** the screen opens over a story save with that byte cleared and an `exanima06` section
- **THEN** the box asks "Do you wish to reset all progress after the portal?" with Yes and No

### Requirement: Reset to portal answer
The box's Yes SHALL delete the slot file the select read and, unless the slot's checkpoint still opens, drop the slot from the list and clear the current slot; the same index SHALL then be selected again. No and the cancel key SHALL close the box and change nothing.

#### Scenario: Yes without a checkpoint
- **WHEN** Yes is chosen for a slot with only its save file
- **THEN** the save file is gone and the list is empty

### Requirement: Slot kind numbers
The slot kinds SHALL be the name builder's: kind 1 SHALL name the save (`.rsg`) and kind 2 the checkpoint (`.rcp`).

#### Scenario: The extension table
- **WHEN** the names of slot 1's two kinds are built with the story prefix
- **THEN** kind 1 is `Exanima001.rsg` and kind 2 is `Exanima001.rcp`

### Requirement: Character screen select kinds
The saves' character screen's select SHALL answer availability bit 2 when the slot's checkpoint opens and bit 1 when its save opens. It SHALL read the checkpoint when it opens and either kind 2 was asked or the save does not open, else the save when it opens.

#### Scenario: Both files
- **WHEN** a slot has a save and a checkpoint and is selected with kind 1, then with kind 2
- **THEN** the availability is 3 both times, the first read is the save and the second the checkpoint

#### Scenario: One file
- **WHEN** a slot has only its checkpoint and is selected with kind 1
- **THEN** the availability is 2 and the checkpoint is read

### Requirement: Continue kind
The saves' screen SHALL open at kind 2 when the first slot's availability is exactly 2, else at kind 1. Its CONTINUE SHALL read the checkpoint first only for kind 2.

#### Scenario: A fresh screen over both files
- **WHEN** the screen opens over a slot with a save and a checkpoint and CONTINUE is chosen
- **THEN** the save is loaded

#### Scenario: The toggle
- **WHEN** the toggle is used on that screen and CONTINUE is chosen
- **THEN** the checkpoint is loaded and copied over the save

### Requirement: Reset to portal file
The reset to the portal SHALL delete the file the select read, so with both files and kind 1 it SHALL delete the save and keep the slot listed for its checkpoint.

#### Scenario: Yes over both files
- **WHEN** Yes is chosen on a fresh screen over a slot with a save and a checkpoint
- **THEN** the save file is gone, the checkpoint stays and the slot stays listed and current

### Requirement: New game slot
The session begin of a new story SHALL create an empty slot in the story manager, and of a new career in the arena manager, before the mode's start; the slot SHALL be named after the created character and become the current slot. Practice SHALL create none.

#### Scenario: A new story
- **WHEN** a story begins over a folder holding slots 1 and 2
- **THEN** slot 3 is created at the front of the story list, named after the created character, and the story's first save writes it

### Requirement: Current manager saves
The flow's save steps SHALL write the current slot of the mode's manager: the story's in story mode, the arena's in arena mode. An arena save SHALL write the company's tables and the manager; the arena SHALL keep no checkpoint.

#### Scenario: A career's save
- **WHEN** a career's slot is current and a save step runs
- **THEN** the arena slot's save holds the company's money and day and no story slot is written

### Requirement: Arena Begin save
The board's Begin and the auto rematch SHALL ask for a save without the death rule before their launch, written before the launch's level change tears the hub down.

#### Scenario: The rematch
- **WHEN** the auto rematch's timer comes due with a board to replay
- **THEN** one save step and one launch are written

### Requirement: Quit autosave
When the application closes while a session plays, the session's quit SHALL run before the app exits, with the menu's page: the story SHALL save with the death rule, the arena SHALL save from the in-game page and forfeit from the arena's pause page; practice SHALL write nothing.

#### Scenario: Closing the app in the story
- **WHEN** the app is asked to close while a story with a current slot plays
- **THEN** the slot is saved in the same frame, and the next frame writes nothing more

### Requirement: Forfeit save
The quit's forfeit SHALL settle the match first and then write the arena slot's player data alone, with no world databases, level section or promotion pass. The pause page's own forfeit SHALL write no save.

#### Scenario: The quit's forfeit
- **WHEN** the app closes during a match from the arena's pause page
- **THEN** the match is settled once and the forfeit's player-data save is asked for

### Requirement: Golem alarm in the settings chunk
A story save's level section SHALL carry the golem alarm in its settings chunk: the armed flag as the gate, the armed point as the tag, the armed zone's pass and the alarm count. A level read from a save's own section SHALL restore it, armed when the point's height is under 1 000 000.

#### Scenario: An armed alarm
- **WHEN** a story save is written while an alarm is armed and that section is read back
- **THEN** the alarm has the same armed flag, point, pass and count

### Requirement: Live journal
Every save SHALL write the in-game journal's pages as its journal chunk, and a story load SHALL put the loaded pages into the in-game journal with the first page selected.

#### Scenario: The journal round trip
- **WHEN** a save is written with two journal pages and then loaded
- **THEN** the save's journal chunk holds both pages and the in-game journal shows them

### Requirement: Running powers in a saved placement
A save SHALL write, after a living character's motion state, a running-powers sub-chunk when the character runs a power: a count word, then the record of each running power whose class saves itself. The count SHALL be the number of powers that wrote, and the sub-chunk SHALL be left out when none wrote.

#### Scenario: The player saved with Sense Minds and a Shell running
- **WHEN** a story save is written while the player runs persistent Sense Minds and Shell
- **THEN** the player's placement ends with the running-powers sub-chunk holding the words 2, the Sense Minds id, 0, the Shell id, 0

### Requirement: Which running powers save
Only the persistent Sense Minds, Mask Mind, Shell and Strikes, and Loyalty and Command Dead SHALL save: the first four write their id and a size of zero; Loyalty and Command Dead write their id, a size of four and their target's character id.

#### Scenario: A thaumaturge running an instant power and Loyalty
- **WHEN** a character running instant Sense Minds and a Loyalty bound to a character is saved
- **THEN** its placement holds one record, the Loyalty's id, 4 and the bound character's id

### Requirement: Only the player's placement saves running powers
Only the save's own placement writer (the player's placement in the player data) SHALL write running powers; the level section's character walk SHALL write none, so a scene character's running powers are not saved.

#### Scenario: A scene thaumaturge running Shell
- **WHEN** a story save is written while a scene character runs Shell
- **THEN** that character's placement in the level section holds no running-powers sub-chunk

### Requirement: Saved running powers rebuilt on load
A character loaded with saved running powers SHALL have each power with a restore row rebuilt: its domain table's cost goes into the body's reserve (three quarters with the reserve technique) and the power holds the same amount. Powers without a restore row SHALL be dropped.

#### Scenario: A thaumaturge with the reserve technique
- **WHEN** a character with the reserve technique is loaded with persistent Sense Minds (cost 0.27) and Calm saved
- **THEN** Sense Minds runs again holding 0.2025, the reserve grows by 0.2025 and Calm is not rebuilt

### Requirement: Loaded powers take their activation
After a loaded character's powers are rebuilt, each SHALL run its activation as a cast does: Sense Minds shows its overlay, Mask Mind sets the masked bit, Shell takes half a blow's energy and Strikes takes its combat factors and the weapon hooks.

#### Scenario: Mask Mind and Strikes loaded
- **WHEN** a character is loaded with persistent Mask Mind and Strikes saved
- **THEN** its status has the masked bit, its swing factor is 3 and Strikes holds both weapon hooks

### Requirement: Loaded bindings relink by character
A loaded Loyalty or Command Dead SHALL come back waiting for its target; on its first tick it SHALL look the target up by the saved character id and, when found and not collapsing, put its marker back on the target with the caster and re-arm the target's behaviour, else end.

#### Scenario: Loyalty bound to a placed guard
- **WHEN** a save holding a Loyalty bound to a character of the scene is loaded
- **THEN** after the first tick the Loyalty runs on that character's body and the character carries the loyalty marker cast by the loaded body

### Requirement: Journey save refreshes the checkpoint
When an adapter journey ends with the player alive, the game SHALL save the current story slot and then copy the save over its checkpoint, replacing an existing checkpoint.

#### Scenario: A journey with an old checkpoint
- **WHEN** the player's journey ends on a slot that already has a checkpoint
- **THEN** the slot is saved and its checkpoint becomes a copy of the save

### Requirement: Slot delete log line
Deleting a slot by its index, from the save screen or by the scan of an outdated save, SHALL log the line `Save delete index: ` followed by the slot's number.

#### Scenario: Deleting the first listed slot
- **WHEN** slot 3, listed first, is deleted from the save screen
- **THEN** the log holds `Save delete index: 3`

### Requirement: Scene load GUI reset
Every scene load SHALL first reset the game's GUI in the original's order: a paused death fade ends and the game time runs again, every game screen and the power tree close, the crystal drain forgets its sources, and the item icon cache empties.

#### Scenario: A level change after a death fade paused the game
- **WHEN** a level change is asked while the death fade has paused the game and the drain holds a source
- **THEN** the fade is over, the game time runs and the drain holds no source

### Requirement: Pending respawn on load
A story load of a save whose transition word is 2 (a respawn pending) SHALL set the word to 1 and load the level with the session-start arrival word. When the level names a respawn portal (its reconstruction device) the player SHALL stand at its frame 15 cm up and settle 16 ticks; otherwise the saved placement SHALL stand and nothing settles. Any other save SHALL settle nothing.

#### Scenario: A save written while a respawn was pending
- **WHEN** such a save is continued
- **THEN** the continue's placement is followed by the portal placement with 16 ticks of settling, and the loaded session's transition word is 1

### Requirement: Reconstruction strip leaves a dressed copy
A scanned player's story death SHALL copy the player's record (left-behind mark 0x40, roles 1 and 0xC2, yellow and red pools 0) into the world character store under its first free id and give the copy to the dead body, which stays dressed. The player SHALL go on with the record stripped of hands, worn and carried items, wearing the underwear, at 0x7FFF.

#### Scenario: A scanned player dies armed and dressed
- **WHEN** the death's strip runs on a player holding a sword and wearing a tunic
- **THEN** the body's record is a world record holding the sword and the tunic, and the kept record holds no item but the underwear

### Requirement: Dead player's body saved into the level
After the respawn or a stripped player's exit clears the body's owned mark, the story save SHALL write the stripped record as the player and the body as a dead character of the level section under its copy's world id, with the copy in the world character store.

#### Scenario: The respawn's save after a death in exanima02
- **WHEN** the respawn's save runs after a strip in exanima02
- **THEN** the exanima02 section holds a dead placement under the copy's id, and a load of that section with the saved world store places the corpse with its gear

### Requirement: Story level changes read the save's section
A story level change SHALL import the destination from the current slot's save when the save holds a section named by its locale, and from the archive otherwise.

#### Scenario: A door back to a saved level
- **WHEN** the player goes back to exanima02 after a save that wrote its section
- **THEN** the load reads the save's exanima02 section

### Requirement: Session arrival words kept across loads
A door's load SHALL name its entrance and the level it leaves, a respawn's load the session-start word; both words are cleared after the load. A pending respawn's save load SHALL set the session-start word and keep it, and every other load SHALL read the word the session last left with no previous level.

#### Scenario: A plain load after a door
- **WHEN** a save is loaded after the session took a door
- **THEN** the load reads arrival word 0 and no previous level

### Requirement: Journal page id words
Each journal page SHALL carry an id word that only the save writer reads. A page the game makes (the opening page, an added page, the erase's reset of the last page) SHALL carry 0; a page a save load puts back SHALL keep its saved word; an erase SHALL move the later pages down with their words. The writer SHALL write each page's word.

#### Scenario: A loaded page keeps its word
- **WHEN** the in-game journal holds a new page and a page with word 0x51, is saved and the save is loaded
- **THEN** the journal chunk holds words 0 and 0x51 and the in-game journal's second page carries 0x51

### Requirement: Door arrival save landing
A door's arrival SHALL restart the player body's stance clip with no blend and no capture, set its tone to full, clear its condition word, condition flags and locomotion flags, probe the floor 300 cm down from 100 cm over the placement (a miss keeps the lifted start), and put the pelvis at 108.18 − 11.1 × the record's height (double precision) plus 13 cm in combat mode or 5 cm over that floor, the held items put back.

#### Scenario: Landing out of and in combat mode
- **WHEN** a body of height 0 arrives through a door, then arrives again in combat mode
- **THEN** its pelvis is asked 113.18 cm, then 121.18 cm, over the probed floor, and its stance clip, condition and flags are reset

### Requirement: Match return reads the saved hub
The settled arena match's return to the hub SHALL read the hub from the current arena slot's saved `arenahub` section when the save holds one, and from the archive otherwise. Other arena loads SHALL leave the archive's level.

#### Scenario: Return after a save that left the hub
- **WHEN** the match returns to the hub and the current arena slot's save holds an `arenahub` section
- **THEN** the hub load reads that section

### Requirement: Older world databases on a load
A save's `#worldobjdb` of an older layout SHALL go through the loader's upgrade chain on a load: `0xDBAC0D08` converts the operative section, `0xDBAC0D07` the weapon and then the operative section, and `0xDBAC0D06` the bodies and the random tables' condition words first. The oldest bodies' paint seeds SHALL be drawn from a run-time library twister seeded from the clock.

#### Scenario: Three older layouts
- **WHEN** story saves whose world database has the magic `0xDBAC0D08`, `0xDBAC0D07` and `0xDBAC0D06` are loaded
- **THEN** the live world tier holds the converted operative in each, the converted weapon from `0D07` on, and the converted body and condition word only from `0D06`

### Requirement: Class change on a load
A saved world instance whose header's material class differs from its template's SHALL take its appearance layers from the template on the load's migration; one of the same class SHALL keep its own.

#### Scenario: Two saved instances
- **WHEN** a save holds two instances of one template, one of another class
- **THEN** after the load only that one's first layer is the template's, its material the set's first

### Requirement: Respawn rebuild
The death screen's CONTINUE SHALL reset the player record's identity (its race byte cleared) and post its rebuild request with no AI start into the world scene. The respawn SHALL, after its save, make the next player spawn from the record that save wrote as the player's and remove the surviving player root.

#### Scenario: A stripped player continues
- **WHEN** a stripped player's CONTINUE and respawn run with a current story slot
- **THEN** the save holds the stripped record as the player, the old player root is gone and the next spawn's record holds no gear while the saved level keeps the corpse

### Requirement: Menu scene
The title after the splash and every return to the title SHALL load `menuscene` with no player before the menu form shows, with the view at (0, 5, 7) m looking at (0, 0, −2) m and a 60° field, and a surviving player's body SHALL stay out of the scene until a level of play is loaded.

#### Scenario: Back to the title
- **WHEN** a story session quits to the title
- **THEN** the menu scene's load is asked for after the world's teardown, the player's body is out of the scene and a new level's load ends the menu view

### Requirement: Transition save
A level exit that keeps a checkpoint SHALL save and copy the save over the checkpoint, overwriting an existing one, only when the current slot's save holds neither an `exanima04` section nor one for the destination; otherwise it SHALL save plainly. The sixth level's clock SHALL be kept only when the copy runs.

#### Scenario: A level entered again
- **WHEN** the exit's destination already has a section in the save
- **THEN** the save is written and the checkpoint is left as it was

### Requirement: Exit level availability
A story exit zone or journey SHALL do nothing when its level is neither in the current slot's save nor in the game archive.

#### Scenario: A missing level
- **WHEN** an exit names a level the archive lacks and the save does not hold
- **THEN** no step is queued

### Requirement: Saves screen paint after the screens' clear
The saves' character screen SHALL paint its continue form after the game screens' draw, which clears the GUI queue each frame, so that its records reach the queue every frame.

#### Scenario: The saves screen is open
- **WHEN** the continue form is open and a frame runs
- **THEN** the GUI queue holds the form's frame with its records

### Requirement: Respawn rebuild holds its record's hands
The player the respawn rebuilds from its stripped record SHALL NOT be given Exarust's carried torch; it holds what the record's hands hold.

#### Scenario: A story respawn
- **WHEN** the death screen's CONTINUE rebuilds the player at the reconstruction device
- **THEN** the new player holds no torch

### Requirement: Respawn rebuild starts at the portal
The rebuilt player's body SHALL start at the respawn placement rather than the level's own start, and the placement SHALL still lay it there once the load ends.

#### Scenario: The rebuilt body's first frames
- **WHEN** the respawn's level is in and the rebuilt player is spawned
- **THEN** its body stands at the respawn portal, not at the level's start
