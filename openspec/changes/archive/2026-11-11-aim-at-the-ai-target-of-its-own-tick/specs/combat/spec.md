## ADDED Requirements

### Requirement: AI target before its attack starters
An AI fighter's attack starters SHALL aim at the combat target it holds when they run: its brain's pick, else its foe while in combat. A target that changed earlier in the same tick SHALL be taken and aimed at before the starters, and an unchanged target SHALL keep the fixed step's aim.

#### Scenario: Stale kept target
- **WHEN** a duellist in combat whose kept combat target is none starts a blow
- **THEN** its combat target is its foe in the same tick

### Requirement: Death ends a running parry
A collapsing fighter that dies while its parry runs SHALL stop parrying: its countered motion SHALL be cleared, its guard SHALL open, both hands SHALL lose the parry's arm bit and every item it holds SHALL have its parry grip disabled. A fighter that dies without a parry SHALL keep these.

#### Scenario: Dying while parrying
- **WHEN** a collapsing fighter with a running parry reaches the death tone
- **THEN** it is dead and no longer parrying, with no countered motion and its guard open
