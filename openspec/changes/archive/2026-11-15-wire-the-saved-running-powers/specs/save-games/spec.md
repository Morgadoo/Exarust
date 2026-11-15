## ADDED Requirements

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
