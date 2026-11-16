## ADDED Requirements

### Requirement: Arc test bits
The simulation SHALL run an emitter's level test only when its record's bits ask for it (0x10000) and its bodies' test only when they ask for it (0x20000). The Shock's burst SHALL ask for both; the hand, weapon, mace and spark arcs for neither.

#### Scenario: A burst and hand arcs on one line
- **WHEN** a burst's tendril and a hand arc's tendril each run through a fighter's limb
- **THEN** the burst shocks the fighter and the hand arcs do not

### Requirement: Arc hand and node frame
An emitter's level and body tests, bounds and mean SHALL take its points as offsets from its hand's place (as they stand for arcs with no hand). The frame SHALL draw them through the node's matrix: for arcs held in a hand, the hand's node offset point; for arcs under a parent that keeps a local frame, the parent's drawn matrix with the node's offset carried through it, each frame. The camera SHALL be taken into the node's frame, and the hum and the voxel light SHALL sit at the node's mean.

#### Scenario: A weapon turned a quarter
- **WHEN** a shock weapon's arcs are drawn while the weapon is turned a quarter about vertical and 1 m along x
- **THEN** the ribbon's last pair straddles the tendril's end turned and moved with the weapon

### Requirement: Arc weapon spheres
A respawn from a weapon's spheres SHALL start the tendril at a sphere more than 10 cm up in the weapon's own frame, unturned.

#### Scenario: A turned weapon
- **WHEN** a weapon turned a quarter respawns a tendril from its one sphere at (0, 20, 5)
- **THEN** the tendril starts at (0, 20, 5)

### Requirement: Arc node lifecycle
The electricity node build SHALL attach the emitter to its node; the emitter's first step SHALL enter it into the world (the local-frame bit from its parent), and a named hum SHALL start silent at pitch 1 at the node. The frame's removal, or the emitter's despawn, SHALL take it out of the world: the hum stops and the light goes.

#### Scenario: The first step
- **WHEN** a burst's emitter on a node of its own steps once
- **THEN** it is in the world, its node keeps the child bit and has no local-frame bit

### Requirement: Arc shared records
An emitter's records SHALL share one buffer of `(segments + 1)·count` points, a segment count apart, so that a record left holding more points than segments by a tail trade runs into the next record's first slots.

#### Scenario: A long trade
- **WHEN** a four-point record of a four-segment emitter trades its tail at point 1 with a four-point branch
- **THEN** the record holds five points and its fifth is the next record's first

### Requirement: Blow wisp placement
A blow's one-strand wisp SHALL be placed as it enters the world: its colour from the engine generator's two draws, running, its head one node up and lit at once.

#### Scenario: A phased blow
- **WHEN** a blow phases through a displaced body
- **THEN** its tendril is running with an alpha of 1, its head at node 1 and its budget 166
