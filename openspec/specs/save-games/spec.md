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
Flushing a scene character SHALL register it when its reference has the top bit set, then remap its worn items, its first hand pair, its second hand pair and its carried items in that order; a carried item holding a hand's old id SHALL take that hand's new id, the first pair looked up first. Loadouts SHALL be left alone.

#### Scenario: Hand reuse
- **WHEN** hands 40, 41, 42, 52, worn 45 and carried 50, 41, 52 are flushed with a remap adding 100
- **THEN** the remap sees 45, 40, 41, 42, 52, 50 and the carried items become 150, 141, 152

### Requirement: Settings chunk on a saved level
The story's level exporter SHALL write the settings chunk only for an old read tag or a saved word above zero (unsigned), carrying the read tag when it was old and the current tag (0, 1 100 000, 0) otherwise.

#### Scenario: Gate
- **WHEN** the tag is current and the saved word is 0
- **THEN** no settings chunk is written
