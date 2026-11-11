## ADDED Requirements

### Requirement: Saved body of a dead character
A story save SHALL write the live pose tree of every scene character who died since the load: each body part's world frame, its bone number and its part children, depth first from the pelvis. A body without parts SHALL write one record at its feet, so the section still exports.

#### Scenario: A killed NPC saved and loaded
- **WHEN** an exanima01 NPC dies, the story is saved and the save is loaded
- **THEN** the level comes from the save's section and the NPC lies dead on its saved 18-record pose where it fell

### Requirement: Saved motion state
A story save SHALL write, for each living scene character an AI drives, its live pose tree and its motion state (the current motion's number and progress). A load SHALL start the saved motion with no blend and put its progress back; a saved attack motion (5 to 12) SHALL load as the defence stance (3) with no progress.

#### Scenario: Idle NPCs saved and loaded
- **WHEN** the story is saved in exanima02 and the save is loaded
- **THEN** the level's NPCs resume motion 0x13 at their saved progress

### Requirement: Saved AI home
A story save SHALL write the AI home (position and heading) of each living guardian whose race word pair is `0x1000`, whose statue byte is clear and which holds a home. A load SHALL set the guardian's home and heading from it and clear its statue byte.

#### Scenario: An awake guardian's home
- **WHEN** an awake golem with a home is saved and the save is loaded
- **THEN** the loaded golem holds the saved home and heading and is no statue

### Requirement: Saved roles
A story save SHALL store each scene character's live roles into its version 7 record, so a role taken while the level ran is kept by a load.

#### Scenario: A follower saved and loaded
- **WHEN** Derrin follows the player to exanima02, the story is saved and the save is loaded
- **THEN** Derrin loads with roles 31, 19 and 1 and follows (activity 4)

### Requirement: Scene content lists at every load
Each level load SHALL classify its class-1 scene nodes and placed objects as the save unit's node hook does: volumes of kind pair `0x1100` or `0x1200` in the first list, bracket-named nodes without a record and volumes of pair `0x2200` in the second, operative objects of class 2 in the third.

#### Scenario: exanima01 loads
- **WHEN** exanima01 loads
- **THEN** its nine door-side volumes are listed
