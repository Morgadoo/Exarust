## ADDED Requirements

### Requirement: Discovery pass cadence
The story's discovery pass SHALL run every frame the story hook reaches it, after the wake-up branches and the once-a-second gate, which gates only the pending level tasks; a dying player's frames SHALL not run it.

#### Scenario: Two frames in one second
- **WHEN** the story hook runs at 100 ms and again at 116 ms for a living player
- **THEN** both frames run the discovery pass
