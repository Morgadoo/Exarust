## MODIFIED Requirements

### Requirement: Character record move
Moving SHALL copy the four hand items into the destination and leave them in the source, and SHALL move the worn list and the carried list with their counts, leaving the source with both lists empty.

#### Scenario: The move
- **WHEN** the source holds hands 1, 3, 4 and 5, worn items 7 and 8 and carried item 10
- **THEN** the destination holds the same hands, worn items and carried item
- **AND** the source keeps its hands
- **AND** the source's worn and carried lists are empty

### Requirement: Creation slot clear
Clearing SHALL find the first of six eight-byte slots whose low word is non-zero and return it. That slot's low word SHALL be zeroed and its bytes 3 through 7, but not byte 2. Every slot after it, the sixth included, SHALL be zeroed whole. With no non-zero word nothing SHALL change and none SHALL be returned.

#### Scenario: The clear
- **WHEN** slot 1 is 1, 2, 3, 4, 5, 6, 7 and 8, slot 5 is all 7 and the others are zero
- **THEN** slot 1 becomes 0, 0, 3, 0, 0, 0, 0 and 0
- **AND** slots 2, 3, 4 and 5 are zero
- **AND** slot 0 is unchanged
- **WHEN** every word is zero
- **THEN** nothing changes and none is returned

### Requirement: Creation defaults
The unknown background's record SHALL start the thaumaturge's powers: two known powers of 16 bytes, the first `0x101` and the second `0x201`, each with `0x5FFF`; the first power slot SHALL hold the first power with bit 20 set and the second slot the second power; the other slots SHALL be cleared, the power word SHALL be `0x510000` and the float beside the slots 4.0.

#### Scenario: The opening
- **WHEN** the powers start
- **THEN** there are two known powers of 16 bytes
- **AND** the first known power is `0x101`, `0x5FFF`, 0, 0
- **AND** the second known power is `0x201`, `0x5FFF`, 0, 0
- **AND** the first two slots hold `0x100101` and `0x201`
- **AND** the float is 4.0 and the power word `0x510000`

### Requirement: Creation continue button
The continue toggle SHALL be labelled "Continue from save" and mirrored when the chosen kind is a save (2), and "Continue from checkpoint" and not mirrored otherwise. It SHALL be enabled only while the kind, compared unsigned, is below the slot's availability mask (bit 0 a checkpoint, bit 1 a save).

#### Scenario: Save, checkpoint and threshold
- **WHEN** the kind is 2 and the mask 3
- **THEN** the label is "Continue from save", the button is mirrored and enabled
- **WHEN** the kind is 1 and the mask 3
- **THEN** the label is "Continue from checkpoint" and the button is enabled
- **WHEN** the kind is 2 and the mask 2, or the kind 1 and the mask 1
- **THEN** the button is disabled

### Requirement: Creation label refresh
The voice preview button SHALL do nothing while the clock, less the last stamp, is below 300, both taken as unsigned 32-bit words and subtracted without wrapping. Otherwise the stamp SHALL take the clock, the voice sample SHALL be loaded when absent, and it SHALL be played.

#### Scenario: Throttled, built and existing
- **WHEN** the clock is 100 and the last stamp 0
- **THEN** nothing runs and the stamp stays 0
- **WHEN** the clock is 400, the last stamp 0 and no sample is loaded
- **THEN** the sample is loaded and played and the stamp becomes 400
- **WHEN** the sample is already loaded
- **THEN** it is not loaded again and is still played
- **WHEN** the clock is 0 and the last stamp is 300 below the 32-bit wrap
- **THEN** nothing runs

### Requirement: Creation link copy
Copying SHALL copy the link word, then compare the stamp plus 2, taken unsigned and added without wrapping, with the clock. At or below it the stamp SHALL take the clock, the pending flag SHALL clear and the callback SHALL run; above it the pending flag SHALL set.

#### Scenario: Pending and firing
- **WHEN** the stamp is 10 and the clock 11
- **THEN** the pending flag sets and the callback does not run
- **WHEN** the stamp is 10 and the clock 12
- **THEN** the stamp becomes 12, the pending flag clears and the callback runs
- **WHEN** the stamp is the last 32-bit word and the clock 0
- **THEN** the pending flag sets

## ADDED Requirements

### Requirement: Creation gear filter
Story background 0 SHALL clear the four hand items, keep only the braies among the worn items and empty the carried list. The arena SHALL clear each of the first two hand items its preset kit does not offer, clear the last two, keep only the braies and empty the carried list. A game mode with its 2 bit set SHALL clear the fist placeholder from the first two hands and empty the carried list. Other modes SHALL be left unchanged. Clearing an item SHALL keep its flags.

#### Scenario: Story, arena and fist
- **WHEN** story background 0 has a sword in hand, braies and a shirt worn and a coin carried
- **THEN** the hands are empty, only the braies are worn and nothing is carried
- **WHEN** the arena's kit offers the first hand's item but not the second's
- **THEN** the first stays, the other three are cleared and only the braies stay worn
- **WHEN** a mode with the 2 bit holds the fist in its first hand
- **THEN** that hand is cleared and nothing is carried

### Requirement: Creation presets
Each story background SHALL keep its preset character under a two-key settings path: the first key is the unknown background's key plus the background (four backgrounds), the second key is the same for all; the arena's career character SHALL use the key below the unknown background's with the same second key. A loaded preset SHALL lose its race byte, known powers and power slots.

#### Scenario: The keys
- **WHEN** the knight's preset is read
- **THEN** its path is the unknown background's first key plus 1 and the shared second key
- **WHEN** a story character is committed for background 4
- **THEN** no preset is saved

### Requirement: Creation background
Loading the unknown background SHALL take its saved preset or a new default record, clear its gear and start its powers. A named background (knight, proctor, villager) SHALL first read its template by name; with a saved preset the preset SHALL keep its own body and skills and take the template's gear, and without one the template SHALL become the record.

#### Scenario: Saved and unsaved
- **WHEN** the proctor has a saved preset
- **THEN** the preset is used and wears the proctor template's gear
- **WHEN** the knight has none
- **THEN** the knight template is the record

### Requirement: Creation entry
Opening the screen SHALL plan its record by screen mode. Recruiting SHALL make a new default record without the gear filter. Story SHALL load the chosen background, clear the record's experience and filter it. Other modes SHALL keep the record they have or run the free creator; a free story or arena creator with no saved preset SHALL get an unfiltered default record. Every record but a continued one SHALL be set to full health.

#### Scenario: Modes
- **WHEN** recruiting
- **THEN** a new record is made, not filtered and set to full health
- **WHEN** the free story creator finds no preset
- **THEN** a default record is made and not filtered

### Requirement: Creation continue entry
Continuing SHALL need the save manager and a valid save list, falling back to the free creator otherwise, and SHALL keep its record unfiltered and its health as it was.

#### Scenario: Continue
- **WHEN** continuing without a save manager
- **THEN** "Save manager not found." is reported
- **WHEN** the save list is invalid
- **THEN** the free creator opens

### Requirement: Creation starting skills
The free creator SHALL write Close Combat, Armour and Shields to 25000 points and their first five techniques (1, 2, 3, 4 and 8; 1, 2, 3, 4 and 5; 1, 2, 3, 4 and 6), leaving the other three skills and each sixth technique as they were. It SHALL do so when it opens and whenever it builds the equipment page of a mode without its own kit rules.

#### Scenario: A used record
- **WHEN** every skill holds 7000 points and technique 9 at every level
- **THEN** Close Combat is 25000 with 1, 2, 3, 4, 8 and 9
- **AND** Ranged Combat still holds 7000 and 9 at every level

### Requirement: Creation experience budget
Spending SHALL cap each skill in order to the experience left, a knight or proctor having at least 10000. A capped skill SHALL clear its techniques from its new level, except that the first capped skill whose technique at that level is chosen SHALL keep it; the technique is read past the slot into the next one when the level exceeds five. What is left SHALL go to the first skill below level 5 with its next technique chosen, up to that next level.

#### Scenario: Caps and leftover
- **WHEN** two skills hold 25000 with all techniques and the budget is 12000
- **THEN** the first holds 12000 with techniques 1, 2 and 3 and the second is empty
- **WHEN** a skill holds 1000 with its first technique and the budget is 30000
- **THEN** it rises to 5000
- **WHEN** a proctor's budget is 9999
- **THEN** 10000 is spent

### Requirement: Creation experience worth
A character's worth SHALL be each skill level at weights 4, 5, 6 and so on in skill order, plus the first skill's partial level at the next weight, summed in 32 bits and quartered toward zero.

#### Scenario: Worth
- **WHEN** one skill holds 25000
- **THEN** the worth is 37500
- **WHEN** one skill holds 7500
- **THEN** the worth is 8125

### Requirement: Creation skills page
The arena SHALL show the skills read-only, clearing a new character's skills from the first used and heading a continued one "Current skill progress". Any other new character SHALL have its skills capped to the account's experience and the page SHALL show "Global experience" with that experience and let the player spend it. The page's tab SHALL read "SKILLS" unless the character is continued.

#### Scenario: Pages
- **WHEN** a new story character opens with 12000 experience
- **THEN** the skills are capped to 12000 and the page shows 12000, editable
- **WHEN** an arena character continues
- **THEN** the page reads "Current skill progress" and is read-only

### Requirement: Creation new global experience
A continued story or free character's page SHALL show "New global experience", read-only: the account's experience converted to worth, less the worth of the last committed character (zero when that covers it), converted back.

#### Scenario: New experience
- **WHEN** a story character continues with 30000 experience and a committed worth of 37500
- **THEN** the page shows 10000 new global experience

### Requirement: Creation starting kit
The apparel kit SHALL come from the arena recruit's table (10 entries) or the career's (15) in the arena, mode 2's alternative table (10) when its flag is set, the unknown story background's (5) in story and none for a named background, the versus table (7) in versus and the default table (6) otherwise. The weapon kit SHALL be none in story, the versus table in mode 2's alternative setup and the weapon table (6) otherwise.

#### Scenario: Tables
- **WHEN** the arena recruits
- **THEN** its 10-entry apparel table is offered
- **WHEN** versus opens
- **THEN** its 7-entry apparel table is offered

### Requirement: Creation kit match
An apparel entry SHALL be offered unless a worn item matches it by id, or by base id and colours; a weapon entry SHALL be offered unless one of the first two hands has its base id.

#### Scenario: Held items
- **WHEN** a worn item's base id and colours match an entry
- **THEN** that entry is not offered

### Requirement: Creation kit item
A kit entry for a plain database item SHALL add the item itself. Otherwise the database SHALL make a new item (the alias target of an item with a colour block, or a new instance) and that item SHALL take the entry's variant, dyes and colours. An alias that makes nothing SHALL add the item unchanged; an instance that cannot be made SHALL add nothing.

#### Scenario: Plain, instance and alias
- **WHEN** a plain entry 12 resolves
- **THEN** item 12 is added with no appearance
- **WHEN** an instance entry gets new item 30
- **THEN** item 30 is added with the entry's appearance

### Requirement: Creation voices
A sex change SHALL set the record's sex bit (2 is female), set the voice prefix to "V" and the appearance label, and reselect voice 0 and variant voice 0 without a preview. A voice sample SHALL be named by the prefix, the one-based voice number in at least two digits and "_GruntS"; the variant sample SHALL insert "DA" before the number. A voice SHALL be taken only when its sample exists; a variant voice SHALL be taken directly for a record with a race byte.

#### Scenario: Names
- **WHEN** a human male selects his first voice
- **THEN** the sample is "VHM01_GruntS" and the variant "VHMDA01_GruntS"

### Requirement: Creation voice pitch and preview
The pitch SHALL be 0.9 plus 0.17 times the slider, worked in doubles and stored as a float. A preview SHALL play at volume 0.4 and avoid repeating the last variant with two or three samples and the last two with four or more, drawing with the engine's generator; one sample SHALL play its first without a draw. The previous voice SHALL stop at zero and the next one SHALL have no bound.

#### Scenario: Pitch and draws
- **WHEN** the pitch slider is 0.001
- **THEN** the pitch is the float `0x3F66718B`
- **WHEN** a one-sample sound plays
- **THEN** the generator does not advance

### Requirement: Creation pages
The appearance page's final button SHALL warn "Please name your character" when a non-continued character has no name. Otherwise the equipment page SHALL be built for the mode (apparel and skills in the arena and story; apparel, or the script's equipment form and a read-only skills panel, in versus; the starting skills, apparel and weapons otherwise) and, for a record without powers, the button SHALL read RECRUIT when recruiting, BEGIN in story, READY in versus and CREATE otherwise.

#### Scenario: Flow
- **WHEN** a named story character goes on
- **THEN** the equipment page holds apparel and skills and the button reads BEGIN

### Requirement: Creation commit
On the equipment page a record with known powers SHALL open the power tree; otherwise recruiting SHALL hand the record to the arena, versus SHALL ready its lobby and any other mode SHALL save the preset, commit the record, store its worth and start the game. The appearance page's back action SHALL leave for the arena when recruiting and for the menu otherwise.

#### Scenario: Commit
- **WHEN** a record with two known powers is committed
- **THEN** the power tree opens
- **WHEN** the appearance page goes back in story
- **THEN** the screen leaves for the menu

### Requirement: Creation continue chooser
Selecting a save slot SHALL ask the save manager what it offers; a zero mask SHALL change nothing, otherwise a missing save SHALL fall back to the checkpoint and a missing checkpoint to the save. The toggle SHALL switch kinds only when the slot offers the other. Stepping and list picks SHALL select the checkpoint of the new slot; a pick of the current slot SHALL do nothing.

#### Scenario: Choices
- **WHEN** a save is chosen on a slot with only a checkpoint
- **THEN** the checkpoint is chosen
- **WHEN** the slot offers nothing
- **THEN** the save stays chosen

### Requirement: Creation save deletion
Deleting SHALL move the selection to the last remaining save, or leave the screen when none is left. The previous and next buttons SHALL be disabled at the ends and the name cut to fifteen characters.

#### Scenario: Deletion
- **WHEN** the last of three saves is deleted
- **THEN** the second is selected
