## ADDED Requirements

### Requirement: Hotkey macros fire from the root task
A bound `KeyDef` macro SHALL run its commands only when its key reaches the root task's key slot with its modifier held and no better shortcut for the key comes first.

#### Scenario: Ctrl and Shift held
- **WHEN** shift+S and ctrl+S are bound and S goes down with both held over the game view
- **THEN** only the shift+S macro runs
