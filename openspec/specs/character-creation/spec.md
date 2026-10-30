# character-creation Specification

## Purpose
The character creation screens: choosing a class and an appearance, rolling attributes, naming the character and carrying that record into a session.

## Requirements

### Requirement: Character record move
Moving SHALL copy the four hand items into the destination and leave them in the source, and SHALL move the worn list and the carried list with their counts, leaving the source with both lists empty.

#### Scenario: The move
- **WHEN** the source holds hands 1, 3, 4 and 5, worn items 7 and 8 and carried item 10
- **THEN** the destination holds the same hands, worn items and carried item
- **AND** the source keeps its hands
- **AND** the source's worn and carried lists are empty

### Requirement: Creation appearance label
The label SHALL be HF when bits 8 through 15 are 0 and bit 0 is set, HM when that band is 0 and bit 0 is clear, HAM for band 0x200, OM for 0x300 and ML for 0x400. Any other band SHALL have no label.

#### Scenario: The bands
- **WHEN** the word is 0x001
- **THEN** the label is HF
- **WHEN** the word is 0x000
- **THEN** the label is HM
- **WHEN** the word is 0x201
- **THEN** the label is HAM
- **WHEN** the word is 0x300
- **THEN** the label is OM
- **WHEN** the word is 0x4ff
- **THEN** the label is ML
- **WHEN** the word is 0x500
- **THEN** there is no label

### Requirement: Creation slot clear
Clearing SHALL find the first of six eight-byte slots whose low word is non-zero and return it. That slot's low word SHALL be zeroed and its bytes 3 through 7, but not byte 2. Every slot after it, the sixth included, SHALL be zeroed whole. With no non-zero word nothing SHALL change and none SHALL be returned.

#### Scenario: The clear
- **WHEN** slot 1 is 1, 2, 3, 4, 5, 6, 7 and 8, slot 5 is all 7 and the others are zero
- **THEN** slot 1 becomes 0, 0, 3, 0, 0, 0, 0 and 0
- **AND** slots 2, 3, 4 and 5 are zero
- **AND** slot 0 is unchanged
- **WHEN** every word is zero
- **THEN** nothing changes and none is returned

### Requirement: Creation appearance settle
Settling SHALL compute `(0.5 − source) × 2`. A distance from the target under the epsilon `0x33ffffd7` SHALL store nothing. Otherwise the value SHALL be stored; a last stamp plus 2 above the clock SHALL set the pending flag, and otherwise the stamp SHALL take the clock and the callback SHALL run.

#### Scenario: Settle, pend and fire
- **WHEN** the source is 0.25 and the target is 0.5
- **THEN** nothing is stored
- **WHEN** the target is 0, the last stamp 100 and the clock 101
- **THEN** 0.5 is stored and the pending flag is set
- **WHEN** the clock is 102
- **THEN** 0.5 is stored, the stamp becomes 102 and the callback runs

### Requirement: Creation appearance copy
Copying SHALL store the source at the target unless their distance is under the epsilon `0x33ffffd7`. A last stamp plus 2 above the clock SHALL set the pending flag, and otherwise the stamp SHALL take the clock and the callback SHALL run.

#### Scenario: Copy, pend and fire
- **WHEN** the source and target are both 0.5
- **THEN** nothing is stored
- **WHEN** the source is 0.5, the target 0, the last stamp 100 and the clock 101
- **THEN** 0.5 is stored and the pending flag is set
- **WHEN** the clock is 102
- **THEN** 0.5 is stored, the stamp becomes 102 and the callback runs

### Requirement: Creation change
Applying a change SHALL set bit 0 of the word when the sex is female and clear it otherwise, take the appearance label, and run the text set, the child call when the child object is present, the global call when the global flag is clear, both passes and the callback. A disabled change SHALL only finalize.

#### Scenario: Female, male and disabled
- **WHEN** the change is enabled, female, the word 0, the child object present and the global flag clear
- **THEN** the word is 1 and the label is HF
- **AND** the text set, child, global, both passes and the callback all run
- **WHEN** the change is enabled, male and the word 1
- **THEN** the word is 0 and the label is HM
- **WHEN** the change is disabled
- **THEN** the word is unchanged and only the finalizer runs

### Requirement: Creation look
Updating the look SHALL set its four values to 16 and the default caption when female, and otherwise set them to 0. A variant below 0 SHALL keep the default caption; otherwise the caption SHALL be the byte `variant + 0x41`.

#### Scenario: Female, lettered and default
- **WHEN** the look is female with variant 0
- **THEN** the four values are 16 and the caption is the default
- **WHEN** the look is male with variant 2
- **THEN** the four values are 0 and the caption is C
- **WHEN** the look is male with variant −1
- **THEN** the caption is the default

### Requirement: Creation appearance scales
Scaling three values SHALL multiply each by 255 and round to the nearest even integer, keeping the low byte. Scaling two values SHALL multiply each by 65535 and round the same way, keeping the low 16 bits.

#### Scenario: Bytes and words
- **WHEN** the three values are 0, 0.5 and 1
- **THEN** the bytes are 0, 128 and 255
- **WHEN** the two values are 0 and 0.5
- **THEN** the words are 0 and 32768
- **WHEN** the value is 1
- **THEN** the word is 65535

### Requirement: Creation variant step
Stepping SHALL read the variant from bits 20 through 23. Below 1 nothing SHALL change. Otherwise the variant SHALL fall by one and be written back into those bits, and when the widget is present its caption SHALL be the default at a new variant of 0 or the byte `(variant − 1) + 0x41`, then the refresh SHALL run.

#### Scenario: Step, default and absent widget
- **WHEN** the word variant is 2 and the widget is present
- **THEN** the word variant becomes 1 and the caption is A
- **WHEN** the word variant is 1 and the widget is present
- **THEN** the word variant becomes 0 and the caption is the default
- **WHEN** the word variant is 0
- **THEN** nothing changes
- **WHEN** the widget is absent
- **THEN** the word steps and no caption is chosen

### Requirement: Creation variant advance
Advancing SHALL read the variant from bits 20 through 23. Above 5 nothing SHALL change. Otherwise the variant SHALL rise by one and be written back into those bits, and when the widget is present its caption SHALL be the byte `(variant − 1) + 0x41` for the new variant, then the refresh SHALL run.

#### Scenario: Advance, cap and absent widget
- **WHEN** the word variant is 2 and the widget is present
- **THEN** the word variant becomes 3 and the caption is C
- **WHEN** the word variant is 5 and the widget is present
- **THEN** the word variant becomes 6 and the caption is F
- **WHEN** the word variant is 6
- **THEN** nothing changes
- **WHEN** the widget is absent
- **THEN** the word steps and no caption is chosen

### Requirement: Creation defaults
The unknown background's record SHALL start the thaumaturge's powers: two known powers of 16 bytes, the first `0x101` and the second `0x201`, each with `0x5FFF`; the first power slot SHALL hold the first power with bit 20 set and the second slot the second power; the other slots SHALL be cleared, the power word SHALL be `0x510000` and the float beside the slots 4.0.

#### Scenario: The opening
- **WHEN** the powers start
- **THEN** there are two known powers of 16 bytes
- **AND** the first known power is `0x101`, `0x5FFF`, 0, 0
- **AND** the second known power is `0x201`, `0x5FFF`, 0, 0
- **AND** the first two slots hold `0x100101` and `0x201`
- **AND** the float is 4.0 and the power word `0x510000`

### Requirement: Creation signature match
Matching SHALL be true when the query equals the signature's id. Otherwise the binding looked up by the query SHALL be required, its id SHALL equal the signature's id, and its four bytes SHALL equal the signature's four bytes.

#### Scenario: Direct, bound and missing
- **WHEN** the query equals the signature's id
- **THEN** the match is true
- **WHEN** the query differs, the binding's id equals the signature's id and all four bytes match
- **THEN** the match is true
- **WHEN** a byte differs
- **THEN** the match is false
- **WHEN** the binding's id differs
- **THEN** the match is false
- **WHEN** there is no binding
- **THEN** the match is false

### Requirement: Creation rail layout
Laying out SHALL store `width − before − unit × count` and return it. Unless compact mode is on, the thumb SHALL be `(count − 4) × unit` and the start `width − (count − 2) × unit`, each in the low 16 bits.

#### Scenario: Full and compact
- **WHEN** the count is 6, the unit 4, the width 100 and the before 10
- **THEN** the view is 66
- **AND** the thumb is 8 and the start 84
- **WHEN** compact mode is on
- **THEN** the view is still 66 and neither rail value is written

### Requirement: Creation continue button
The continue toggle SHALL be labelled "Continue from save" and mirrored when the chosen kind is a save (2), and "Continue from checkpoint" and not mirrored otherwise. It SHALL be enabled only while the kind, compared unsigned, is below the slot's availability mask (bit 0 a checkpoint, bit 1 a save).

#### Scenario: Save, checkpoint and threshold
- **WHEN** the kind is 2 and the mask 3
- **THEN** the label is "Continue from save", the button is mirrored and enabled
- **WHEN** the kind is 1 and the mask 3
- **THEN** the label is "Continue from checkpoint" and the button is enabled
- **WHEN** the kind is 2 and the mask 2, or the kind 1 and the mask 1
- **THEN** the button is disabled

### Requirement: Creation part search
Finding SHALL search a node's attachments only when its kind masked with `0x89` equals `0x89` and its wanted count is not negative. An attachment tagged `0xFCA0` SHALL match. Otherwise the search SHALL continue into the node's children, and a leaf without a match SHALL return false.

#### Scenario: Attachment, child and miss
- **WHEN** a node of kind `0x89` with wanted count 2 has attachments `0x100` and `0xFCA0`
- **THEN** the search is true
- **WHEN** the wanted count is −1 and the only attachment is `0xFCA0`
- **THEN** the search is false without children
- **WHEN** a child matches
- **THEN** the search is true
- **WHEN** no node or attachment matches
- **THEN** the search is false

### Requirement: Creation frame selection
Selecting SHALL do nothing when the requested frame is below 1 or already equals the current frame plus one. Otherwise the current frame SHALL become the requested frame less one, the part search result SHALL be stored and the refresh SHALL run.

#### Scenario: Change, no change and no request
- **WHEN** the requested frame is 3, the current frame 5 and the part found
- **THEN** the current frame becomes 2, the part is stored and the refresh runs
- **WHEN** the current frame is already 2
- **THEN** nothing changes and no refresh runs
- **WHEN** the requested frame is 0
- **THEN** nothing changes

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

### Requirement: Creation list rebuild
Rebuilding SHALL build a list from the index and fail when the new list's count is not positive. Otherwise it SHALL store the index, release an existing list, store the new one, set the child's index, caption a present widget with the byte `index + 0x41` and draw when allowed.

#### Scenario: Build, empty and suspended
- **WHEN** the index is 2, an old list exists, the new count is 3, the caption widget is present and drawing is allowed
- **THEN** the index is stored, the old list is replaced and the new one stored
- **AND** the child index is 2, the caption is C and the draw runs
- **WHEN** the new count is 0
- **THEN** the rebuild fails and nothing is stored or drawn
- **WHEN** drawing is suspended
- **THEN** the list is rebuilt and not drawn

### Requirement: Creation second list rebuild
Rebuilding the second list SHALL store the index at `0x1a4` and the child's `0xa4` and stop when the band is non-zero. Otherwise it SHALL build the list, fail on a non-positive count, store the index, replace the old list, run the sub-routine, caption widget 9 with `index + 0x41` and draw when allowed.

#### Scenario: Banded, built and empty
- **WHEN** the band is non-zero
- **THEN** the index is stored and no list is built or drawn
- **WHEN** the band is zero, the index is 1, the count 3 and an old list exists
- **THEN** the index is stored, the list replaced, the caption B and the draw runs
- **WHEN** the count is 0
- **THEN** the rebuild fails and nothing is stored or drawn

### Requirement: Creation record reset
Resetting SHALL set the five words at `0x64` through `0x84` and the tail at `0xf0` to zero, then run the reset with size 8.

#### Scenario: The reset
- **WHEN** the record resets
- **THEN** the five words and the tail are zero
- **AND** the reset size is 8

### Requirement: Creation slot defaults
The slot table SHALL hold three eight-byte entries with the low word `0x61A8`, bytes 1, 2, 3 and 4, and byte 6 equal to 8, 5 and 6.

#### Scenario: The slots
- **WHEN** the slots are filled
- **THEN** the first is `0xA8`, `0x61`, 1, 2, 3, 4, 8, 0
- **AND** the second is `0xA8`, `0x61`, 1, 2, 3, 4, 5, 0
- **AND** the third is `0xA8`, `0x61`, 1, 2, 3, 4, 6, 0

### Requirement: Creation deferred callback
Flushing SHALL do nothing unless the pending flag is set and the stamp plus 1 is below the clock. Then the stamp SHALL take the clock, the flag SHALL clear and the callback SHALL run. The result SHALL be true either way.

#### Scenario: Waiting, firing and idle
- **WHEN** the pending flag is clear
- **THEN** nothing fires and the result is true
- **WHEN** the flag is set, the stamp 10 and the clock 11
- **THEN** nothing fires
- **WHEN** the flag is set, the stamp 10 and the clock 12
- **THEN** the stamp becomes 12, the flag clears and the callback runs

### Requirement: Creation view attach
A tip SHALL be released only when present, and a view SHALL be stored and its callback attached only when present.

#### Scenario: Present and absent
- **WHEN** the tip is present
- **THEN** it is released
- **WHEN** the view is absent
- **THEN** nothing is stored or attached

### Requirement: Creation link copy
Copying SHALL copy the link word, then compare the stamp plus 2, taken unsigned and added without wrapping, with the clock. At or below it the stamp SHALL take the clock, the pending flag SHALL clear and the callback SHALL run; above it the pending flag SHALL set.

#### Scenario: Pending and firing
- **WHEN** the stamp is 10 and the clock 11
- **THEN** the pending flag sets and the callback does not run
- **WHEN** the stamp is 10 and the clock 12
- **THEN** the stamp becomes 12, the pending flag clears and the callback runs
- **WHEN** the stamp is the last 32-bit word and the clock 0
- **THEN** the pending flag sets

### Requirement: Creation body attach
Attaching SHALL store the body, its camera and its link, give a present pane the body's `0x1f0` and a present link the renderer's `0x30`.

#### Scenario: Pane and link
- **WHEN** the pane and link are present
- **THEN** the body, camera and link are stored and both targets are written
- **WHEN** the pane is absent
- **THEN** its target is not written

### Requirement: Creation view size
Sizing SHALL take the frame plus one, or one with no frame, store the unit times 12 and tick when the global is present.

#### Scenario: With and without a frame
- **WHEN** the frame is 3 and the unit 4
- **THEN** the count is 4 and the size is 48
- **WHEN** there is no frame
- **THEN** the count is 1
- **WHEN** the global is present
- **THEN** the tick runs

### Requirement: Creation view refresh
Refreshing SHALL run the link, camera, first and second ticks.

#### Scenario: The refresh
- **WHEN** the view refreshes
- **THEN** the link, camera, first and second ticks all run

### Requirement: Creation view flag
Setting the flag SHALL set bit `0x10000` of the word when the flag is set and clear it otherwise, then refresh and store the part result.

#### Scenario: Set and clear
- **WHEN** the flag is set and the word is 0
- **THEN** the word is `0x10000`, the refresh runs and the part result is stored
- **WHEN** the flag is clear and the word is `0x10000`
- **THEN** the word is 0

### Requirement: Creation scale apply
Applying SHALL store `source × 0.17 + 0.9` at the child's scale and link. With the special flag, a clear global byte SHALL run the global call, a missing label SHALL be built and the label SHALL be drawn.

#### Scenario: Plain and special
- **WHEN** the source is 0 and the special flag is clear
- **THEN** the scale is 0.9 and only the store runs
- **WHEN** the source is 1, the special flag is set, the global byte is clear and no label exists
- **THEN** the scale is 1.07
- **AND** the global call, the label build and the draw all run
- **WHEN** the label already exists
- **THEN** it is not built and it is still drawn

### Requirement: Creation list step
Stepping SHALL rebuild the list at the index less one when stepping back and the index is positive, and at the index plus one when stepping forward. The index SHALL be returned with whether the rebuild ran.

#### Scenario: Back, forward and blocked
- **WHEN** the index is 2 and the step is back
- **THEN** the rebuild runs at 1
- **WHEN** the index is 0 and the step is back
- **THEN** no rebuild runs
- **WHEN** the index is 2 and the step is forward
- **THEN** the rebuild runs at 3

### Requirement: Creation mode switch
Switching SHALL store the object as the global and call its routine in mode 2. Any other mode SHALL clear bit `0x10` of the looked-up widget for a positive mode and set it otherwise.

#### Scenario: Special and widget
- **WHEN** the mode is 2
- **THEN** the global is stored and the special call runs
- **WHEN** the mode is 1
- **THEN** the widget's bit is cleared
- **WHEN** the mode is 0
- **THEN** the widget's bit is set

### Requirement: Creation mode toggle
Toggling SHALL leave mode 2 for mode 1 with flag bit 1 and enter mode 2 from any other mode with flag bit 2, refreshing. Without the bit nothing SHALL change.

#### Scenario: Down, up and refused
- **WHEN** the mode is 2 and bit 1 is set
- **THEN** the mode becomes 1 and the refresh runs
- **WHEN** the mode is 1 and bit 2 is set
- **THEN** the mode becomes 2 and the refresh runs
- **WHEN** the needed bit is clear
- **THEN** the mode does not change and no refresh runs

### Requirement: Creation close
Closing SHALL, when the child is inactive, clear the stored byte and schedule the special form in mode 10, else the regular form. Otherwise it SHALL release the first present of `0x208`, `0x220` (also calling `0x2d`) and `0x218`, then `0x238` and `0x210`; outside mode 12 it SHALL release `0x228` and `0x1d0` and reset two words. It SHALL then tick, clear the active word, write the strings and clear the widget bit.

#### Scenario: Inactive and active
- **WHEN** the child is inactive and the mode is 10
- **THEN** the stored byte clears and the special form is scheduled
- **WHEN** the child is active, `0x208` is absent, `0x220` and `0x218` are present, `0x210` and `0x228` are present and the mode is 12
- **THEN** only `0x220`, `0x210` and the `0x2d` call run
- **AND** the widget reset does not run
- **WHEN** the mode is not 12
- **THEN** the widget reset and both later releases run

### Requirement: Creation option hit
Hitting SHALL be false for key 8 or a missing list; otherwise it SHALL be the hit result, and a hit SHALL select the last entry. The result SHALL be true.

#### Scenario: Hit, key 8 and no list
- **WHEN** the key is 3, the list is present and the hit test succeeds
- **THEN** the hit is true, the last entry is selected and the result is true
- **WHEN** the key is 8
- **THEN** nothing is hit or selected
- **WHEN** the list is missing
- **THEN** nothing is hit or selected

### Requirement: Creation singleton
The singleton SHALL be created when absent and returned otherwise.

#### Scenario: First and later
- **WHEN** the singleton is absent
- **THEN** the creator runs and the singleton is ready
- **WHEN** the singleton is present
- **THEN** the creator does not run and the singleton is ready

### Requirement: Creation form close
Closing the form SHALL clear the stored byte and schedule the special form in mode 10, otherwise the regular form.

#### Scenario: Special and regular
- **WHEN** the mode is 10
- **THEN** the stored byte clears and the special form is scheduled
- **WHEN** the mode is 9
- **THEN** the regular form is scheduled

### Requirement: Creation key
A key with a selection SHALL forward and return the base result. Without one it SHALL return `0x40` and close.

#### Scenario: With and without a selection
- **WHEN** a selection exists and the base returns 3
- **THEN** the key returns 3 and does not close
- **WHEN** no selection exists
- **THEN** the key returns `0x40` and closes

### Requirement: Creation show
Showing SHALL prepare, dispatch, refresh and build when the check passes.

#### Scenario: Build and skip
- **WHEN** the check passes
- **THEN** the prepare, dispatch, refresh and build all run
- **WHEN** the check fails
- **THEN** the prepare, dispatch and refresh run and the build does not

### Requirement: Creation capture
Capturing SHALL copy six words, compute `current − last + saved`, store the mode, clear the stored byte and schedule.

#### Scenario: The capture
- **WHEN** the current is 10, the last 4 and the saved 7
- **THEN** the value is 13
- **AND** the six words, the mode and the scheduled flag are stored

### Requirement: Creation page
Paging SHALL do nothing unless the maximum exceeds the index. Otherwise the mode SHALL become 1, the index SHALL rise by one and the refresh SHALL run.

#### Scenario: Page and stop
- **WHEN** the maximum is 5 and the index 2
- **THEN** the mode becomes 1, the index 3 and the refresh runs
- **WHEN** the maximum is 2 and the index 2
- **THEN** nothing changes

### Requirement: Creation select
Selecting SHALL use the wanted index less one. An unchanged index SHALL do nothing; otherwise the mode SHALL become 1 and the refresh SHALL run at that index.

#### Scenario: Select and stay
- **WHEN** the wanted index is 4 and the current 2
- **THEN** the mode becomes 1, the index 3 and the refresh runs
- **WHEN** the wanted index is 3 and the current 2
- **THEN** nothing changes

### Requirement: Creation table selection
Selecting the table SHALL store the source index less one and apply that entry.

#### Scenario: The selection
- **WHEN** the source index is 4
- **THEN** the stored index is 3 and the entry is applied

### Requirement: Creation session start
Starting the session SHALL store the value, clear the stored byte and schedule.

#### Scenario: The start
- **WHEN** the session starts
- **THEN** the value is stored, the byte clears and the schedule runs

### Requirement: Creation menu dispatch
The menu dispatch SHALL take the alternate call when the global byte is set and the primary call otherwise.

#### Scenario: Alternate and primary
- **WHEN** the byte is set
- **THEN** the alternate call runs
- **WHEN** the byte is clear
- **THEN** the primary call runs

### Requirement: Creation screen start
Starting the screen SHALL reset, reset again when the condition holds, create the form when absent and prepare it. In mode 12 it SHALL set the form mode to 2 with the arena flag and 1 otherwise, refresh, build when checked and tick, then store the form and the value 2.

#### Scenario: Arena, story and other
- **WHEN** the mode is 12 and the arena flag is set
- **THEN** the form mode is 2, the refresh runs, the build runs and the tick runs
- **AND** the form and 2 are stored
- **WHEN** the mode is 12 without the arena flag
- **THEN** the form mode is 1
- **WHEN** the mode is not 12
- **THEN** the refresh, build and tick do not run and the form is still stored

### Requirement: Creation apply step
The step SHALL relink, refresh the camera and run both ticks. It SHALL store the part result only when a target exists.

#### Scenario: With and without a target
- **WHEN** a target exists and the part is found
- **THEN** the link, camera, store, part and both ticks all run
- **WHEN** the target is absent
- **THEN** the store is skipped and the rest run

### Requirement: Creation part refresh
The part refresh SHALL store the search result at the target only when the target exists.

#### Scenario: With and without a target
- **WHEN** the target exists and the part is found
- **THEN** the result is stored and the part is found
- **WHEN** the target is absent
- **THEN** nothing is stored

### Requirement: Creation destroy
Destroying SHALL run the base destructor and free on positive flags. A child above one SHALL be decremented and one at one or below SHALL be released, and the base SHALL always run.

#### Scenario: Decrement, release and free
- **WHEN** the flags are 1 and a child with count 2 exists
- **THEN** the base destructor runs, the child decrements, the base runs and the object is freed
- **WHEN** the child count is 1
- **THEN** the child is released
- **WHEN** the flags are 0
- **THEN** the base destructor and the free do not run

### Requirement: Creation point copy
The copy SHALL take the child's three words into the object's point.

#### Scenario: The copy
- **WHEN** the child's point is 1, 2 and 3
- **THEN** the object's point is 1, 2 and 3

### Requirement: Creation resume
Resuming SHALL save the global list, refresh at the current index and call the resume slot.

#### Scenario: The resume
- **WHEN** the screen resumes
- **THEN** the save, the refresh at the current index and the resume call all run

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
Opening the screen SHALL plan its record by screen mode. Recruiting SHALL make a new default record without the gear filter. Story SHALL load the chosen background, clear the record's experience and filter it. Other modes SHALL keep the record they have or run the free creator. Every record but a continued one SHALL be set to full health by four `0xffff` halves, and every filtered record SHALL then get the underwear pass.

#### Scenario: Modes
- **WHEN** recruiting
- **THEN** a new record is made, marked, not filtered and set to full health
- **AND** its yellow, red, focus and save seed words are `0xffff`
- **WHEN** the free story creator finds no preset
- **THEN** a default record is made, marked and not filtered

### Requirement: Creation new-character mark
A record the screen makes SHALL be marked new: always when recruiting, when the screen had none in the free creator and in continue, and in story only for a fresh record that reads a saved preset or starts the unknown background. A named background without a preset adopts its template unmarked.

#### Scenario: Marks
- **WHEN** story opens a named background with no preset and no record
- **THEN** the template is adopted and not marked
- **WHEN** story reads a saved preset with no record
- **THEN** the fresh record is marked

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
An apparel entry SHALL be offered unless a worn item matches it by id, or by base id and the entry's four colour-layer bytes compared in layer order; a record without a colour-layer block SHALL NOT match by base. A weapon entry SHALL be offered unless one of the first two hands has its base id.

#### Scenario: Held items
- **WHEN** a worn item's base id and its four layer bytes match an entry
- **THEN** that entry is not offered
- **WHEN** only the fourth layer byte differs
- **THEN** the entry is offered

### Requirement: Creation kit item
A plain kit entry SHALL add the item itself. An item with an appearance block whose alias marker is `@` SHALL add the alias target with the entry's variant and three tint bytes; any other marker SHALL add the entry unchanged. Any other item SHALL become a new instance with the entry's variant, its four colour-layer bytes (each layer marked with five) and the second and third tint bytes as dyes where non-zero; an instance that cannot be made SHALL add nothing.

#### Scenario: Plain, instance and alias
- **WHEN** a plain entry 12 resolves
- **THEN** item 12 is added with no appearance
- **WHEN** an instance entry with tint 2, 0, 4 and layers 5, 6, 7, 8 gets new item 30
- **THEN** item 30 takes layers 5, 6, 7, 8 and dye bytes unchanged, 4
- **WHEN** an alias entry with marker `@` resolves to item 21
- **THEN** item 21 takes the variant and the three tint bytes

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

### Requirement: Creation preview frame
The creation preview SHALL sit 180 units in and 24 down in its parent, the parent's width less 360 wide and its height less 28 high, without clamping.

#### Scenario: A 1024 × 768 parent
- **WHEN** the parent is 1024 × 768
- **THEN** the preview is at (180, 24), 664 × 740

### Requirement: Creation layouts
Form mode 12 SHALL take the ten-element layout table and build no editor sub-forms; every other mode SHALL take the seven-element table and build both editors.

#### Scenario: Mode 3
- **WHEN** the form opens in mode 3
- **THEN** it has seven layout elements and the editors

### Requirement: Creation drops
A drop onto the creation preview SHALL ignore item 8, go to the mode's own list when the form has one, to the apparel list in game mode 8, to the hook in game mode 0x1010, and otherwise by the item's kinds in order: apparel 0x1000 and weapons 0x4000 to their lists with the placing step, 0x8000 to the weapon list without it.

#### Scenario: An item of kind 0x8000
- **WHEN** an item whose record has only kind 0x8000 is dropped outside game modes 8 and 0x1010
- **THEN** it goes to the weapon list and is not placed

### Requirement: Centred slider weight
The system SHALL set a centred creation slider's weight to its position less one half, negated, so the middle gives a negative zero and the ends -0.5 and +0.5.

#### Scenario: Slider in the middle
- **WHEN** a centred slider is at 0.5
- **THEN** its weight is a negative zero

### Requirement: Decomposition maps and layers
The system SHALL name the decomposition passes' maps by their shipped unit order (diffuse: humandcmpw, ancientdifm, skindecomp, humandcmpf; specular: humandcmpw, ancientdifm, humandcmpg; normal: ancientnrmm0, humandcmpn) and SHALL cap the normal pass at four layers.

#### Scenario: More than four weights
- **WHEN** five normal layers carry weights
- **THEN** only the first four are blended

### Requirement: Creation back switch
The equipment page's tab SHALL hide only the first built of the weapons panel, the skills panel and the versus lobby form, sliding the skills panel by `0x2d` only when it is the one hidden, then hide the versus skills panel and the versus equipment form when they exist. A non-continued character SHALL show its two appearance panels again and set both tab words to `0x110`. From the appearance page the tab SHALL leave the creator: to the arena for a recruit, otherwise to the menu.

#### Scenario: Back
- **WHEN** weapons and skills are both built
- **THEN** only the weapons panel is hidden and nothing slides
- **WHEN** only the skills panel is built
- **THEN** it is hidden and slides by `0x2d`
- **WHEN** a recruit leaves from the appearance page
- **THEN** the arena's form opens

### Requirement: Creation preview routines
The preview material SHALL be applied, when the generator exists, to items 0 through the list's highest index. The icon sheet's icon size SHALL be eight GUI units and the dummy-head entry height two more. The continue view SHALL refresh the first listed item on each of its passes only when that item's record carries flag `0x100`, and the extra item on each pass when it exists. The sex radio SHALL act only when checked and SHALL open the audio device only when it was not ready.

#### Scenario: Loops
- **WHEN** the item list's highest index is -1
- **THEN** no item gets the material
- **WHEN** the highest index is 3
- **THEN** four items get it
- **WHEN** the GUI unit is 5
- **THEN** the icon size is 40

### Requirement: Creator reached from the menu
The main menu's BEGIN SHALL open the story background picker, and confirming a card SHALL open the creator for that background. The practice rows and the in-game NEW GAME SHALL open the free creator in practice mode, NEW CAREER in arena mode and VERSUS in versus mode. Opening SHALL hide the menu.

#### Scenario: A story character
- **WHEN** the player picks BEGIN and confirms the knight's card
- **THEN** the creator opens in story mode for background 1 with the menu hidden

#### Scenario: A practice character
- **WHEN** the player picks EXPERT on the practice page
- **THEN** the creator opens in practice mode for the expert ladder

### Requirement: Creator layout
The creator SHALL fill the window and lay out the creation form's seven records, the body editor's twelve and the hair editor's seven at unit 4, then scale every place and size by the live GUI unit as the original's rescale does (divided by 4, times the unit). The body editor SHALL sit at 4, 60, the hair editor at the form's width less 176, 32, and the preview viewport 180 in and 24 down, the form's width less 360 wide and its height less 28 high.

#### Scenario: Unit 4 on a 1024 × 768 window
- **WHEN** the creator opens at 1024 × 768 with unit 4
- **THEN** the body editor is at 4, 60, 172 × 592, the hair editor at 848, 32 and the viewport at 180, 24, 664 × 740
- **AND** BACK and NEXT are 164 × 28, 8 in from the bottom left and bottom right corners

### Requirement: Creator commits the record
NEXT SHALL show the name warning when the name is empty and otherwise switch to the equipment page; the equipment page's final button SHALL write the creator's name, gear, skills and full health into the record and start the story (BEGIN) or the practice ladder (CREATE), the player spawning with the record's physique, skin, hair and name.

#### Scenario: An unnamed character
- **WHEN** NEXT is pressed with an empty name
- **THEN** the "Please name your character" warning shows and the page does not change

#### Scenario: Begin the story
- **WHEN** a named story character's BEGIN is pressed
- **THEN** the record carries the name and full pools and the story starts at exanima01

### Requirement: Saves' character screen
The menu's CONTINUE rows SHALL open the creation form in its continue mode with the original's eleven-record continue table, showing the first listed slot's character; Previous, Next and the slot combo box SHALL select other slots, Delete SHALL ask in the `Delete Character` box before deleting the slot, and the story's toggle SHALL switch between the save and the checkpoint when both exist.

#### Scenario: Two slots
- **WHEN** the screen opens over two story slots
- **THEN** the newest slot's character shows, Previous is disabled and Next is enabled

#### Scenario: Deleting the last slot
- **WHEN** the only slot is deleted through the box
- **THEN** both of its files are gone and the screen returns to the menu

### Requirement: Story save load
A story save SHALL hold the player's record and placement, and the screen's CONTINUE SHALL load the slot into the running game: the player spawns from the saved record, dressed from its items, at the saved placement on the save's level, with the saved world clock.

#### Scenario: Save and continue
- **WHEN** a story session is saved to a new slot and that slot is continued
- **THEN** the restored record has the saved name, worn and carried items, and the placement the saved position and heading

### Requirement: Creation equipment panels
The apparel panel SHALL be bound as the current equipment form as soon as it is made, before the mode's table is picked, and an existing panel SHALL be shown and bound without consulting the table; only a made panel with a table SHALL be filled and finished. The skills page SHALL hide the second preview panel only when it exists, flag the skills panel with 0x20000 only when it makes it, and rebind an existing panel to the edited record.

#### Scenario: A named story background
- **WHEN** the apparel panel is made for a named story background
- **THEN** it is the current equipment form with no items and no finish

#### Scenario: The skills page again
- **WHEN** the skills page opens with the panel already made
- **THEN** the panel gains no flag and its record is rebound

### Requirement: Creation continue page texts
Opening the continue page SHALL, when the save manager's list has entries, select the first of the list's count in the label, show the first entry's display string and put the first entry's name in the name box, keeping row 0; an empty list SHALL clear both texts and leave the kept row.

#### Scenario: Three saves
- **WHEN** the page opens on three entries
- **THEN** the label selects 1 of 3 with the first display string and the name box shows the first name
