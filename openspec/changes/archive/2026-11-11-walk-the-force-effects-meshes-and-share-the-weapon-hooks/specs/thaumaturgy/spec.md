## ADDED Requirements

### Requirement: Blast level mesh walk
A thrown blast SHALL meet each level mesh the static grid offers round it: each live chunk whose reach `reach·2 + margin + 10` passes the mesh's bounding sphere and box SHALL be pushed out of every face whose plane is within that reach on either side, and a chunk met SHALL move half way to its pushed centre and keep nine tenths of its weight.

#### Scenario: A blast chunk over the floor
- **WHEN** a chunk of reach 4 is 10 cm over a floor
- **THEN** it is pushed to 18 cm, ends at 14 cm and keeps 0.9 of its weight

### Requirement: Bolt sampled walk
A thrown bolt's chunk SHALL take the reach `reach·2 + margin`; one that moves at least that reach a tick SHALL be swept along its move from where it is through the mesh's swept test, whose record end stays the mesh frame's origin, and one that moves less SHALL ask the front sphere test where it is. Each face met SHALL add its push to the chunk, and a met chunk SHALL lose its weight and be worn 0.015.

#### Scenario: A slow bolt chunk
- **WHEN** a bolt chunk of reach 4 moving 5 cm a tick is 10 cm over a floor
- **THEN** it is pushed to 28 cm, its weight is 0 and its wear 0.015

### Requirement: Two-sided part query
A blast's, a wave's and a torrent's part contact SHALL ask the part's node mesh the two-sided sphere test, so a chunk or point within reach behind a face meets it too.

#### Scenario: A chunk behind a face
- **WHEN** a blast chunk is 5 cm behind a part's face within its reach
- **THEN** the part is struck and the chunk is pushed further behind

### Requirement: Wave on the blast's contacts
A thrown force wave SHALL meet the level's meshes, the cloths and each other body part by part through the blast's contacts, and a struck character SHALL take the blast's knock, instability counter, concern and loosened pins.

#### Scenario: A wave at a standing duellist
- **WHEN** a wave is thrown at a duellist who stands still
- **THEN** the duellist's parts are struck and the duellist is knocked

### Requirement: Blast charge loop
A blast or bolt SHALL start the `thmfrc_cu01` loop at its node with volume 0 and pitch 1, its start `length·0.8·r` into the buffer, level it by the frame's charge volume, and free it once thrown past a charge of 0.9.

#### Scenario: A charging blast
- **WHEN** a blast starts charging
- **THEN** the charge loop plays and rises with the charge

### Requirement: Blast release sound
A thrown blast SHALL play its release set (`thmfrc_bb`, the bolt's `thmfrc_blt`) once on its release handle at the tick's release pitch, levelled by the setup's release volume, and both voices SHALL be freed when the effect leaves.

#### Scenario: A thrown bolt
- **WHEN** a bolt is thrown
- **THEN** `thmfrc_blt` plays once at the release pitch

### Requirement: Torrent mesh contact
A running torrent's live points SHALL meet the level's meshes and each other body's parts through the two-sided sphere test, gathering the surface points and pushes per stream; a point met SHALL ease toward its pushed place and raise the stream loop's volume and pitch by its life share.

#### Scenario: A torrent at a wall
- **WHEN** a torrent point within reach of a level face
- **THEN** it eases half way to its pushed place

### Requirement: Torrent limb callback
A torrent's part contact SHALL push the part's particles at the weights of the gathered surface mean by the limb impulse from the point's pushed place, and a struck character SHALL take the torrent's knock per part.

#### Scenario: A torrent at a crate
- **WHEN** a torrent point meets a crate's face
- **THEN** the crate's particles are pushed and the point eases toward its pushed place

### Requirement: Shared weapon hooks
Force Strikes' activation SHALL take the body's item and strike hooks, Shock Weapons' the item hook, and either's end SHALL clear both; weapon changes and landed blows SHALL reach only the power holding the slot.

#### Scenario: Shock Weapons ends under Strikes
- **WHEN** Shock Weapons ends while Force Strikes runs
- **THEN** Strikes' aura no longer ripples on a landed blow

### Requirement: Barricade floor cast
A cast barricade's floor probe SHALL meet only the static meshes of floor nodes.

#### Scenario: A barricade placed over a floor
- **WHEN** a barricade is cast with a floor node 10 cm below the probe
- **THEN** the wall's middle is 100 cm above that floor

### Requirement: Focus item slot
When a caster's hands change, either hand's weapon section naming ability type 4 SHALL set the body's focus slot bit, a character's persistent slots SHALL be counted again, and the bit going SHALL tell the running powers past the slots to end.

#### Scenario: Dropping the focus item
- **WHEN** a character with three slots without the item and four running powers puts the focus item away
- **THEN** the fourth running power is told to end
