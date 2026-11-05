# Spec Delta

## MODIFIED Requirements

### Requirement: Creation commit
On the equipment page, a record with known powers SHALL open the power tree. Otherwise:
- recruiting SHALL hand the record to the arena;
- versus SHALL hand the record to its lobby form, and with no lobby form READY SHALL do nothing;
- any other mode SHALL save the preset, commit the record, store its worth and start the game.

The appearance page's back action SHALL leave for the arena when recruiting. Otherwise it SHALL leave for the menu through the creator's cancel.

#### Scenario: Commit
- **WHEN** a record with two known powers is committed
- **THEN** the power tree opens
- **WHEN** the appearance page goes back in story
- **THEN** the screen leaves for the menu

#### Scenario: READY without a lobby
- **WHEN** a named versus character's READY is pressed with no lobby form
- **THEN** the creator stays on the equipment page and no session starts
