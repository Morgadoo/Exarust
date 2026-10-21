## MODIFIED Requirements

### Requirement: Turned blow
A blow on a body whose state flag 0x400 is set SHALL deal nothing (no health, no push, no daze) and SHALL make the striker recoil, whatever its size; the blow sag the armour pass set before the turn SHALL stay.

#### Scenario: Turned
- **WHEN** the tests' cut lands on a body that turns blows
- **THEN** its health is whole, the only shove carries no push and no daze but the blow sag min(2·I, 1), and the striker's swing recoils at strength 0.5

## ADDED Requirements

### Requirement: Swing end rounding
The end a recoil or a follow-through writes SHALL be its scale times 400 rounded half to even, not truncated, plus the clock (and the recoil's 200 ms).

#### Scenario: A short recoil
- **WHEN** a striker recoils with a drive of 0.013 at 1000 ms
- **THEN** its stance returns at 1201 ms (0.52 rounds to 1)

#### Scenario: A glancing follow-through
- **WHEN** a body blow at 0.009 cm a tick with gain and recoil 1 is followed through at 1000 ms
- **THEN** it ends at 1001 ms

### Requirement: Stab pierce without a number
A stab whose energy is no number SHALL take the pierce's root branch: no pierce while the shove is nothing, never the thrust's √2.

#### Scenario: A lost contact
- **WHEN** the tests' stab lands with no number for its contact
- **THEN** its penetration, wound and shove are 0
