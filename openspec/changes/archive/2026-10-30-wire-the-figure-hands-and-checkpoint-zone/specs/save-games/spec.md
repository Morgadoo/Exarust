# Spec Delta

## ADDED Requirements

### Requirement: Checkpoint script
A script's `CheckpointSave` call made for the player while the settings' checkpoints are on SHALL run the checkpoint step on the current story slot (the save, then its copy over the checkpoint when the slot has none); a call for another character, or with checkpoints off, SHALL write nothing.

#### Scenario: Load inside the checkpoint zone
- **WHEN** a story slot without a checkpoint is loaded with the player standing in `exanima01`'s checkpoint zone
- **THEN** the save is written and then copied, and the checkpoint's bytes equal the save's

#### Scenario: Another character
- **WHEN** the call reports a character that is not the player
- **THEN** neither file changes

### Requirement: Loaded character request
After a story or arena load clears the player record's race byte, the load SHALL make the content stream's request for the loaded character with the request word `0x1000` (no AI start, no saved AI pose restored).

#### Scenario: Story load
- **WHEN** a story slot with a player record is restored
- **THEN** the restore carries a request with word `0x1000`
