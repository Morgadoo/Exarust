## ADDED Requirements

### Requirement: Held item attached after its holder's body
An item equipped before its holder's body is built SHALL have its body built only once the holder's body exists, at its grip under the hand where that body put it. Its grip SHALL take its holder key before the grip builder runs, and the builder SHALL not tie a grip whose holder is not yet known. A body SHALL be built from its skeleton's nodes only, not from the parts of the dressing's merged mesh.

#### Scenario: A sword equipped before its body
- **WHEN** a weapon is equipped on a character whose body is built in a later frame
- **THEN** on the first tick with both bodies, the hand part grips the item, its holder key is the body's first part, and it is not dropped

## MODIFIED Requirements

### Requirement: Height Query mesh mode
The ground placement's Height Query SHALL take, besides the level, the body meshes listed in the nine sector cells around its point. It SHALL leave out a body out of the world and a living character's limb, and keep a dead character's limb. It SHALL also leave out the items the placed body holds. Each mesh SHALL be pre-tested by the segment against its box and the swept sphere against its bounding sphere, then swept down its tree's column. The highest surface met SHALL win.

#### Scenario: A crate on the floor
- **WHEN** a Height Query of radius 10 cm comes down over a 40 cm crate body standing on the floor, and another beside it
- **THEN** the first finds 40 cm and the second the floor

#### Scenario: Its own sword under a foot
- **WHEN** a body is placed on the ground while its held sword's blade lies over one foot
- **THEN** the sword is not taken for ground and the pelvis is placed over the floor
