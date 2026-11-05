## ADDED Requirements

### Requirement: Bridged fixed step by default
The game SHALL run its fixed step in the simulation's world behind the bridge unless asked not to, and the bridged run SHALL follow the one-world run within the run-to-run variation the one-world run has with itself.

#### Scenario: The arena duel
- **WHEN** the arena duel runs 900 fixed frames with joint traces, in one world twice and behind the bridge
- **THEN** over the first 100 ticks the bridged trace differs from the first one-world trace no more than the second one-world trace does

### Requirement: Query filters bridged
Every type a moved system's query filters name SHALL be bridged, and the coverage SHALL list any that is not.

#### Scenario: A marker filter
- **WHEN** a moved system's query asks for entities with a marker component
- **THEN** the marker is bridged, so the query matches the same entities in the simulation's world

### Requirement: Step digest
On request the game SHALL write, after every fixed step, a hash of every digestible bridged type's values that does not depend on entity ids, in one world and behind the bridge alike.

#### Scenario: Two one-world runs
- **WHEN** two runs write the digest with fixed frames and game time held while loading
- **THEN** the types without hash maps or frame-dependent state hash alike step by step
