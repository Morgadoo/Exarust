## ADDED Requirements

### Requirement: Scene body generator
The game SHALL build an arena fighter, a company entrant or a hub NPC from a spawn plan on the placed characters' spawn path, and SHALL remove a body named by its root or its body model together with its root, subtree and physics capsule. Frames without a command SHALL not read the game's tables.

#### Scenario: Spawn and despawn
- **WHEN** a generic opponent plan at level 2, tier 1 is sent with the game's packs and three frames run, and then a despawn names the built body model
- **THEN** one body is built with its physical body, the root and model are gone after the despawn, and the physics world holds one body fewer

### Requirement: Live arena rounds
A match's first round SHALL stand every layout slot except the player's own entry; the end screen's Continue SHALL keep a living company fighter and the opponent that won, clear the dead, and bring the reserve or a fresh opponent; the match's end SHALL settle the result into the career and ask for the hub with the saved return placement.

#### Scenario: Reserve against the winner
- **WHEN** an Elimination match at round 2 with one loss continues, with a standing opponent and a dead company fighter
- **THEN** round 3 starts, the dead fighter is despawned, the opponent stays and one reserve body is asked for

### Requirement: Hub bodies
When the hub is entered, in arena mode or under the play view's practice stand-in, the game SHALL place the manager and ask once for the shopkeeper at -320, 0, -110 cm facing 15 degrees and one body per employed merchant or physician at their planned spots; story mode SHALL leave the hub alone.

#### Scenario: Practice stand-in
- **WHEN** the hub level and the player are present with the flow in practice mode, and again in story mode
- **THEN** in practice mode the manager is placed and the hub bodies are asked for; in story mode neither happens
