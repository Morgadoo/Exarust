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
