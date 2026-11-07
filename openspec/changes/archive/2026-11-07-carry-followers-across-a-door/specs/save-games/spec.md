## ADDED Requirements

### Requirement: Followers travel through a door
A level change to a named level that no world reset accompanies SHALL take every scene character whose applied activity is the follow (4) along, each once, before the old level is torn down: its record with its live possessions and health, its local items promoted to the world tier, and its held roles. A change behind a world reset, or a teardown with no destination, SHALL carry nobody.

#### Scenario: A save load
- **WHEN** a save is loaded while a companion follows
- **THEN** no follower is carried into the loaded level

### Requirement: Followers land around the player
Once the destination's collision exists each travelling follower SHALL be placed on the stored follower offsets around the player's landing (turned by the placement's heading, else the identity), on its own floor probe, and respawned with the roles it held and its carried health.

#### Scenario: A recruited companion takes a door with the player
- **WHEN** Derrin follows the player in exanima01 and the player changes to exanima02 through a door
- **THEN** Derrin is respawned in exanima02 at the first offset, (−100, 0, −100) cm from the player with no portal matrix, and still follows
