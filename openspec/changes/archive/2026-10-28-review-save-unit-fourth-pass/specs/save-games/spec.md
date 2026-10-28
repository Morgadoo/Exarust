## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Scene character flush
Flushing a scene character SHALL register it when its reference has the top bit set, then remap its worn items, its first hand pair, its attached scene nodes holding a first-pair saved id, its second hand pair and its carried items in that order; a carried item holding a hand's old id SHALL take that hand's new id, the first pair looked up first. Loadouts SHALL be left alone. A repeated migration address SHALL be reported and not added to the travelling list.

#### Scenario: Hand reuse
- **WHEN** hands 40, 41, 42, 52, worn 45 and carried 50, 41, 52 are flushed with a remap adding 100
- **THEN** the remap sees 45, 40, 41, 42, 52, 50 and the carried items become 150, 141, 152

#### Scenario: Attached nodes
- **WHEN** attached nodes 40, 41, 99 are flushed with that remap
- **THEN** they become 140, 141, 99
