## ADDED Requirements

### Requirement: Archive pass on a level load
A level read from the archive SHALL run the tier migration and then the world-load walk over its own records before its characters are dressed, and the records the pass changes SHALL be what the level's characters and the live database read.

#### Scenario: exanima01 from the archive
- **WHEN** exanima01 loads for a new visit
- **THEN** its apparel records have no paint layer still to colour and no variant still to roll

### Requirement: Archive pass entropy
Each palette, layer and weapon roll of the archive pass SHALL start from the next word of a fixed sequence, so the same level loads the same way twice.

#### Scenario: Two loads
- **WHEN** the same level is loaded twice from the archive
- **THEN** its walked records are byte-identical

### Requirement: Outfit palette
The arena's profile outfit pass SHALL roll the item palette around the profile's two company colours when its flag 8 is set and at random otherwise, at kind 2 for a rank-five fighter with an empty off hand; the level pass SHALL roll it at random.

#### Scenario: A company outfit
- **WHEN** a profile with flag 8 and colours 0x2F and 0x57 is dressed
- **THEN** the outfit's palette has primary 0x2F and secondary 0x57

### Requirement: Company armour mode
The max-rank armour of a company outfit SHALL take mode two when the palette matched a metal colour to the company's secondary, and mode three otherwise; the outfit's draws SHALL not change.

#### Scenario: No metal near the secondary
- **WHEN** a company palette matches no metal colour
- **THEN** the max-rank armour takes mode three

### Requirement: One item value
The AI's search SHALL weigh weapons and apparel with the same desirability and protection routines as the item copies, the protection's bone factor `w·(√(c·0.01)·0.005)` in double precision.

#### Scenario: A covered bone
- **WHEN** a piece covers one bone of weight 2 with 100 and has protection bytes summing 40
- **THEN** its protection value is 0.4
