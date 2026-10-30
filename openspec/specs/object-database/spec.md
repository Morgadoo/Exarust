# object-database Specification

## Purpose
The object database's three tiers (templates, a level's own objects, the world's) and their string libraries: how they are read, written back, upgraded from older layouts, and changed by instancing, duplication, promotion, removal and template migration, with the packed item fields and amounts the unit shares.

## Requirements

### Requirement: Database framing
The object database reader SHALL count `table_len / 16 − 1` records, read nothing past the header when that is below one, and keep the table, the slot after it and the payload blob as the file holds them, without sorting or compacting. A record of size 0, or whose payload lies outside the blob, SHALL have no view.

#### Scenario: An empty database
- **WHEN** a file is the magic and a one-slot table length with nothing after it
- **THEN** it reads as an empty database

### Requirement: Database write-back
The writer SHALL save the magic, a table length of `(records + 1) × 16` and, when there is a record, the table, the slot after it and the blob as held. Every shipped database SHALL write back byte for byte: `objdb.rdb`, the 22 level databases in `Resource.rpk` and the 20 actor databases in `actors.rpk` (those of an older layout with the current magic).

#### Scenario: Payloads out of order
- **WHEN** a database's blob holds record 5's payload before record 2's, with eight unused bytes between them and a stale slot after the table
- **THEN** writing it gives back the same bytes

### Requirement: Table search and insertion
Lookups SHALL binary-search the table comparing ids by their 32-bit difference; insertion SHALL place a new empty entry by the same comparison and refuse a duplicate id; the first free id SHALL be the smallest positive id the scan in table order does not pass.

#### Scenario: Ids past 2^31
- **WHEN** id `0x80000001` is inserted into a table holding id 1
- **THEN** it goes first, as its 32-bit difference from 1 is negative

### Requirement: Payload store and deletion
Storing a payload of the same size SHALL overwrite it in place; another size SHALL remove the old bytes, moving every entry whose payload starts after them down, and append the payload at the blob's end. Deleting SHALL store an empty payload, close the table up and leave in the slot after the table the deleted entry when it was the last, else the new last entry.

#### Scenario: A longer payload
- **WHEN** records 1, 2 and 3 hold 32 bytes each from offset 0, and record 1 is stored with 36 bytes
- **THEN** records 2 and 3 move to offsets 0 and 32 and record 1 goes to offset 64

#### Scenario: Deleting the middle record
- **WHEN** record 2 of three is deleted
- **THEN** the slot after the table repeats record 3's entry

### Requirement: Older layouts
A `0xDBAC0D08` database SHALL have its operative sections converted to the 60-byte layout (class and kind through the original's tables, a relay keeping its kind). A `0xDBAC0D07` one SHALL first convert its weapon sections to 96 bytes; a `0xDBAC0D06` one SHALL before that convert its bodies and random tables. Each conversion SHALL replace the old section by an appended new one; unknown magics SHALL be errors.

#### Scenario: An old key
- **WHEN** a `0xDBAC0D08` record holds an operative section of class 2 and kind 3
- **THEN** after loading it holds a 60-byte operative section of class 4 and kind 7 as its last section

#### Scenario: A body of the middle layout
- **WHEN** a `0xDBAC0D07` record holds a body and a random table
- **THEN** both are left as they were

### Requirement: String library framing
The string library reader SHALL follow the database count rule and keep the slot after the table and the text area, and its writer SHALL save them as held; every shipped library (`objstrings.rdb` and the 22 level libraries) SHALL write back byte for byte. Ids SHALL be one-based; id 0 and an entry of length 0 SHALL give no text.

#### Scenario: A level without own strings
- **WHEN** a level library is the magic and a one-slot table length alone
- **THEN** it reads as empty and writes back as the same eight bytes

### Requirement: String library routines
Allocation SHALL take the first slot without references from the free hint on, else append one. Setting text of the slot's length SHALL overwrite it; of another length SHALL remove the old text, moving later entries down, and append; empty text SHALL release the slot. A release reaching no references SHALL zero the slot, leave its text as a hole and raise the free hint to the slot when higher.

#### Scenario: Replacing a longer text
- **WHEN** a library holds "Sword", "Two-handed Sword" and "Maul" in order and entry 2 is set to "Axe"
- **THEN** the text area is "SwordMaulAxe" and every entry still reads its own text

### Requirement: Id tiers
Ids with bit 31 SHALL name the local tier, else bit 30 the world tier, else the base tier, by their low 30 bits (the base tier by the whole id).

#### Scenario: Both tags
- **WHEN** an id carries both bit 31 and bit 30
- **THEN** it names the local tier

### Requirement: Display strings
A model section's name or description id of 0 SHALL give none; exactly `0x80000000` SHALL read the same field of the record the payload's parent word names, in the shared library; another id with bit 31 SHALL read the record's tier library; any other id the shared library. A chosen id without text SHALL give an empty string. Every own string of the shipped level databases (119) SHALL resolve in its level's library.

#### Scenario: An inheriting instance
- **WHEN** a local record whose parent is template 10 ("Sword", "A sword.") has both model ids `0x80000000`
- **THEN** its name is "Sword" and its description "A sword."

### Requirement: Instancing
Instancing an untagged base record whose header flags have bit 0 SHALL copy it into the local tier under the first free id with its table flags and both model string ids inheriting. Its header SHALL name its parent when the base tier holds that id with a named model, copying that template's occupancy, slot and coverage blocks into the copy's body, else itself. A tagged id SHALL be migrated and returned.

#### Scenario: A marked record naming a template
- **WHEN** base record 20 (marked, parent 30) is instanced and record 30 has the model "Tpl" and coverage 50 at bone 1
- **THEN** local record 1 names 30, has coverage 50 at bone 1 and inherits both strings

### Requirement: Local duplication
Duplicating a local instance with header flag bit 0 SHALL copy it with its string ids, adding a reference to each own string, its table flags read by the source's index after the insertion; one without the bit SHALL clone the template its header names.

#### Scenario: Duplicating between ids
- **WHEN** local ids 1 and 3 exist and record 3 is duplicated
- **THEN** the copy is local id 2 and has table flags 0

### Requirement: Promotion
Promoting a local object SHALL return its recorded world id when the id remap table holds it; otherwise it SHALL promote its container children first (a count at body `0x14`, 12-byte entries from `0x18`) and rewrite their ids, move each own string with text to the world library, insert it under the first free world id with its table flags and delete it locally. A missing local object SHALL give 0. Promoting every object of every shipped level SHALL leave its local tier empty.

#### Scenario: A box with a child
- **WHEN** local box 2 holds local child 1 and is promoted with the id remap table on
- **THEN** the child becomes world 1 and the box world 2, naming world 1 at body `0x18`

### Requirement: Remap tables
A promotion SHALL record its pair in the id remap table and, for an operative object of kind 0, in the link remap table, each when on; switching a table on or off SHALL empty it. The link rewrite SHALL replace each local door's linked object (door body `+4`) through the link table, then turn it off and empty it.

#### Scenario: A door and its key
- **WHEN** a local door links local key 1 (kind 0) and the key is promoted with the link table on
- **THEN** the link rewrite changes the door's link to world 1 and a second pass changes nothing

### Requirement: Removal
Removing a local or world object SHALL keep it when it has a random table section, and otherwise release in its tier's library each model id whose low 31 bits are not zero and delete it; removing an untagged id SHALL do nothing. Deleting SHALL do the same for any tier without the random-table test. Clearing a tier SHALL leave its library.

#### Scenario: A plain id in a local record
- **WHEN** a local record names own string 1 and plain id 2 and is removed
- **THEN** local library slots 1 and 2 are both freed

### Requirement: Migration of models, weapons, containers and doors
Migrating a local or world object (tag exactly bit 31 or bit 30) SHALL refresh each section of tag `0x1000` or more its template also has. The model SHALL take, with a changed material class, the template's first 40 bytes with inheriting ids, else its name, flags and bytes `0x1C`–`0x1F`. A weapon SHALL take the seeds when the template's are `0xFFFFFFFF`, the flags' low 28 bits, and the stat bytes when it has seeds and byte `0xF` is 0. Containers and doors SHALL take their words and `0x200` flag.

#### Scenario: A weapon without seeds
- **WHEN** a weapon copy whose word at `+4` is 0 migrates
- **THEN** its stat bytes stay as they were

### Requirement: Migration of bodies
A body SHALL refresh, with a changed class, its appearance layers with the material-set check. A template paint seed of `0xFFFFFFFF` SHALL set the copy's, copy bytes `0x1B0` and `0x1B1`, set each layer's byte 7 to `0x88` for materials 1–25 and the power bytes by the template's `0x80` bits; an instance seed of `0xFFFFFFFF` SHALL take the template's first `0x224` bytes. The fixed blocks SHALL then copy and the condition refresh run. Migrating every shipped level SHALL stay total.

#### Scenario: A pristine copy's value
- **WHEN** a copy of quality 255 and no damage migrates from a template of quality 255 worth 1000
- **THEN** its value is 1250 and a protection byte of 100 becomes 130

### Requirement: Packed fields
The packed setters SHALL round to nearest with ties to even and OR their values in unmasked as the original's. The signed field SHALL keep the count bits, clear bits 2–7, store round(|31v|) above the count, a magnitude past 31 setting the sign bit, then the sign; the three-level fractions of byte 3 SHALL store n = round(8v), above 1 as (n >> 1) + 1 capped at 5 (3 in the two-bit field).

#### Scenario: Half a step
- **WHEN** the signed field is set to 0.5
- **THEN** it holds 16 (15.5 rounded to even), not 15

#### Scenario: A value past the field
- **WHEN** the signed field is set to 1.5
- **THEN** its byte is `0xB8` and it reads back negative

#### Scenario: A positive value after a negative one
- **WHEN** a field holding count 3 and a negative value is set to 1.0
- **THEN** its byte is `0x7F` and it reads back 1.0

### Requirement: Amounts
Amounts SHALL pack as units of 1, 12 and 144 with the word cut to 16 bits; adding units or subtracting another amount SHALL clear the amount at zero or below.

#### Scenario: Taking too much
- **WHEN** 10 units have an amount of 3 grosses subtracted
- **THEN** the amount is zero

### Requirement: Item-use test
The use test SHALL give 0 without an operative section and for classes 2, 4, 8 and 0, except 2 for class 0 of kind `0x11` when bits 16–23 of the caller's word are clear; 1 for class 1 of kind 8, 9, 10 or 14 with those bits set; and 2 otherwise.

#### Scenario: A key
- **WHEN** an object of class 1 and kind 9 is tested with bit 16 set, then clear
- **THEN** the results are 1 and 2

### Requirement: Object manager scene trees
A record with a model name SHALL get the factory weapon its `@` names or the model; without one, the container volume of its volume section (radius the length of its extents, x and z squared first) or, with an operative section, a 50 cm placeholder of radius 86.60254; otherwise no tree.

#### Scenario: A volume
- **WHEN** a record without a model has a volume section of 30, 40 and 120
- **THEN** its tree is a volume of radius 130

#### Scenario: A placed operative without a model
- **WHEN** a level places an operative record with no model name
- **THEN** the running game spawns its placeholder node at the placement

### Requirement: Object manager flags and materials
Every tree the manager makes SHALL carry node flag 0x4000 and each child 0x5000 with the parent's hidden bit. A weapon record SHALL hide its `*` materials; an item-body record SHALL hide `*` and paint `&` materials, and a painted tree SHALL not be kept as its model's master. A release that finds another tree in the slot SHALL report the mismatch and still hand the slot on.

#### Scenario: Materials
- **WHEN** an item-body record's model has materials `*mask` and `&cloth` and a plain record's has `*glass`
- **THEN** the mask is hidden, the cloth painted and the glass drawn

### Requirement: Oldest body conversion
The `0xDBAC0D06` body converter SHALL keep the blocks below `0x1A0`, copy the power word from the old bytes `0x1AC`–`0x1B3`, map the old category to the armour type (1–8 to 1, 2, 3, 1, 2, 4, 5, 6; others keep the byte), apply the category's finish changes when the finish is 1–7 without bit 7, draw one paint seed per body as a 63-bit value modulo `0xFFFFFFFF`, and turn the four old 16-byte parts into the first four appearance layers.

#### Scenario: An unknown category
- **WHEN** an old body's category byte is 9
- **THEN** the converted armour type is 9

### Requirement: Oldest random-table condition words
After the bodies, each random-table row's condition word SHALL be rewritten by the setters applied in turn to the same word. An unrolled word (bit 7 clear) SHALL become `0x5284`. A rolled one SHALL take wealth 1, then the damage read from the updated word as the sign-magnitude value of bits 0–2 plus 2 quarters, then the spread from bits 4–6 of the word after that, then dirt 0.5 and spread 0.25.

#### Scenario: A rolled condition word
- **WHEN** an old row's condition word is `0x00F0`
- **THEN** it becomes `0x57B4`

### Requirement: Use action and experience table
Using a class-0 object SHALL hand its header name to the reader for kind 7 and close the item container and open the divination-card form for kind 5; other kinds and classes do nothing. The experience amount for a level SHALL be 10, 20, 50, 100, 200, 500, 1000 and 2000 for levels 0–7 and 2000 above.

#### Scenario: A letter
- **WHEN** an object of class 0 and kind 7 is used
- **THEN** its header name goes to the reader

### Requirement: Placed light share precision
A condition-scaled item's placed light share SHALL be (1 − c·(1/255))² computed in double precision and cast once to single, c being bits 16–23 of the placement flags.

#### Scenario: Condition 27
- **WHEN** a condition-scaled item is placed with condition 27
- **THEN** its share has the bits `0x3F4CA884`

### Requirement: Factory weapon migration
Migrating a weapon that takes its template's stat bytes and whose model section (visited before it) names a factory with a leading `@` SHALL hand the name after the `@` and the weapon body to the caller's factory pass when there is one. Then, with or without a pass, it SHALL scale the stat bytes by the weapon's own quality and damage (capped at 160) and rescale the model's value word with the weapon's own quality as the base.

#### Scenario: A factory weapon with no factory
- **WHEN** a weapon copy of quality 0 and no damage, whose template model is `@sword_lnga` with stat bytes 9, migrates without a pass
- **THEN** its slash, crush and pierce bytes are 7, 8 and 7 and its value word is rescaled by 1.25

#### Scenario: A plain weapon
- **WHEN** a weapon whose model name has no `@` migrates with a pass
- **THEN** the pass is not called and the stat bytes stay the template's

### Requirement: Factory pass write-back
The item side's factory pass SHALL, for a body with no stored choices and only its first seed, copy that seed to the second, set the stored and rerolled marks and write the initial choices (picked parts' selection bytes, the first three areas). For a random roll with a first seed it SHALL set the stored mark and write every part's selection byte and the first four areas. It SHALL then write the recipe's stat bytes.

#### Scenario: First seed only
- **WHEN** a body with flags 0 and seeds 0x12345678 and 0 passes through a factory with no parts
- **THEN** both seeds read 0x12345678, the flags are the stored and rerolled marks and the selection bytes are unchanged

### Requirement: Editor look roll
The object editor's look roll SHALL draw in order: a top bit choosing one of the families 0, 4, 8, 14, 19, 25, 28 or any of 32; the first shade 7 − round(6r²) from the unit draw r; the second colour (a family, shade 7 or 6); a third shade of 3 or 4; with the full flag, a one-in-five trim row (else 3, 0); the light byte; and a finish of 3–4 one time in three, else 83–85. It SHALL clear two editor bytes.

#### Scenario: Seed zero
- **WHEN** the roll runs with the full flag from generator state 0 and a unit draw of 0.36
- **THEN** the first colour is 6, the second 223, the light 1, the finish 84 and the state 0x5F4B0359

### Requirement: Character data record copies
A record copy SHALL take the source record's payload and table flags into the target tier under its first free id tagged with the target tier, SHALL re-make in the target library each own model string id above the inherit value whose text the source tier holds, and SHALL write the inherit value for one whose text is gone; other ids and missing records SHALL come back unchanged.

#### Scenario: Own string re-made
- **WHEN** a local record whose model names own string "Local name" is copied into the world tier
- **THEN** the copy is the world tier's first free id and its name id names "Local name" in the world library

### Requirement: Character data instance copies
An instance copy SHALL first make the instance, SHALL read an own string from the instance's tier and any other from the shared library, SHALL always take a target slot, and SHALL return the new id tagged local whatever the target tier.

#### Scenario: Instance copy tag
- **WHEN** a marked local record is copied as an instance into the world tier
- **THEN** the returned id is tagged local and the record lives in the world tier

### Requirement: Migration of a foreign store
The template migration SHALL run over a store held outside the database with the templates resolved in the database, rewriting only records whose header names a template that resolves.

#### Scenario: Class change outside the database
- **WHEN** a store holds an instance whose template's material class differs
- **THEN** the migrated instance's two string ids are the inherit value and the database is unchanged

### Requirement: Armour weight and loadout ranks
An item's armour weight SHALL sum, over its twenty coverage words that are at least one, the bone's weight times sqrt(coverage × 0.01) × 0.005 (in double, narrowed once) times the sum of the four protection bytes, accumulated in single precision. A hand item's rank SHALL be the low five bits of the weapon section's word at 0x42 when the record has a weapon section, else of the shield section's word at 0x08; a worn item's rank SHALL be the low five bits of the body's word at 0x1AB.

#### Scenario: Quarter coverage on a heavy bone
- **WHEN** bone 9 (weight 2.5) has coverage 25 and the protection bytes sum to 100
- **THEN** the armour weight is 0.625

#### Scenario: Short weapon section
- **WHEN** a record has a weapon section too short for its rank word and a shield ranked 3
- **THEN** its hand rank is 0
