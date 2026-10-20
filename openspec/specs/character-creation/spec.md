# character-creation Specification

## Purpose
The character creation screens: choosing a class and an appearance, rolling attributes, naming the character and carrying that record into a session.

## Requirements

### Requirement: Character record move
Moving SHALL copy the four words, the count, the tail and both lists into the destination, take both source lists and zero the source's count and tail, leaving the source's words unchanged.

#### Scenario: The move
- **WHEN** the source has words 1, 2, 3 and 4, count 5, parts 7 and 8, tail 9 and tail parts 10
- **THEN** the destination has the same words, count, parts, tail and tail parts
- **AND** the source has count 0, tail 0 and empty lists
- **AND** the source keeps words 1, 2, 3 and 4

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
Clearing SHALL find the first of six eight-byte slots whose low word is non-zero and return it. That slot's low word SHALL be zeroed and its bytes 3 through 7, but not byte 2. Each slot after it up to the fifth SHALL be zeroed whole. With no non-zero word nothing SHALL change and none SHALL be returned.

#### Scenario: The clear
- **WHEN** slot 1 is 1, 2, 3, 4, 5, 6, 7 and 8 and the others are zero
- **THEN** slot 1 becomes 0, 0, 3, 0, 0, 0, 0 and 0
- **AND** slots 2, 3 and 4 are zero
- **AND** slot 0 and slot 5 are unchanged
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
Opening SHALL clear 40 bytes at `0x198`, set the count to 2 and the stride to 16, and make two zeroed entries. The first entry SHALL get `0x101` and `0x5FFF` with `0x100000` ORed into the first word; the second SHALL get `0x201` and `0x5FFF`. The first and second entry words SHALL be stored, then 4.0 and `0x510000`.

#### Scenario: The opening
- **WHEN** the object opens
- **THEN** the count is 2 and the stride 16
- **AND** the first entry is `0x100101`, `0x5FFF`, 0, 0
- **AND** the second entry is `0x201`, `0x5FFF`, 0, 0
- **AND** the stored words are `0x100101` and `0x201`
- **AND** the value is 4.0 and the flags word is `0x510000`

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
The button SHALL be labelled "Continue from save" and set bit `0x40000000` when the mode is 2, and "Continue from checkpoint" with the bit clear otherwise. Bit `0x10` SHALL be set when the mode is not below the threshold.

#### Scenario: Save, checkpoint and threshold
- **WHEN** the mode is 2 and the threshold 3
- **THEN** the label is "Continue from save", the bit is set and the button is disabled
- **WHEN** the mode is 5
- **THEN** the label is "Continue from checkpoint", the bit is clear and the button is enabled
- **WHEN** the mode equals the threshold
- **THEN** the button is enabled

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
Refreshing SHALL do nothing while the clock is less than 300 above the last stamp. Otherwise the stamp SHALL take the clock, the label SHALL be built when no widget exists, and the widget SHALL be drawn.

#### Scenario: Throttled, built and existing
- **WHEN** the clock is 100 and the last stamp 0
- **THEN** nothing runs and the stamp stays 0
- **WHEN** the clock is 400, the last stamp 0 and no widget exists
- **THEN** the label is built, the widget is drawn and the stamp becomes 400
- **WHEN** the widget already exists
- **THEN** the label is not built and the widget is still drawn

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
Copying SHALL copy the link word, then compare the stamp plus 2 with the clock. At or below it the stamp SHALL take the clock, the pending flag SHALL clear and the callback SHALL run; above it the pending flag SHALL set.

#### Scenario: Pending and firing
- **WHEN** the stamp is 10 and the clock 11
- **THEN** the pending flag sets and the callback does not run
- **WHEN** the stamp is 10 and the clock 12
- **THEN** the stamp becomes 12, the pending flag clears and the callback runs

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
