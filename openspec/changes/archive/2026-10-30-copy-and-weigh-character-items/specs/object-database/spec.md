# Spec Delta

## ADDED Requirements

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
