## ADDED Requirements

### Requirement: Apparel records as the search reads them
For an item record with an apparel body, the search SHALL read 20 occupancy words at 0x10, 20 coverage words at 0x150, the four protection bytes at 0x1A0, the special byte at 0x214, and the model section's byte at 0x1C. The apparel body SHALL be read before any weapon section. Bytes past a short section SHALL read 0.

#### Scenario: A worn piece
- **WHEN** a record has an apparel body covering bone 10 at 75 with protection 10, 20, 30, 40, and a model byte of 51
- **THEN** the search weighs it as apparel with those values, and once worn it leaves no wear room for a piece that overlaps it

### Requirement: Weapon records as the search reads them
For an item record with no apparel body, the search SHALL read the weapon section's flags word, quality (0x0C), damage (0x0D), thrust (0x33), tier byte (0x42, of which only the low five bits count) and special word (0x44). Without either section, an operative or container part SHALL make the item worth gathering. A record with none of these SHALL not be gathered.

#### Scenario: A sword record
- **WHEN** a weapon section holds flags 0x11, quality 200, damage 5, thrust 70 and tier byte 0xE3
- **THEN** the search weighs it as a one-handed weapon of 3 points: (70/140 + 3·((200/255)²/2 + 1/2)·(1 − 0.3·5/255))·0.8

### Requirement: Following the player in the running game
A level brain whose roles choose the follow SHALL follow the player. Inside the personal stop it SHALL stand. Inside the band (400 cm; 280 cm while following a route; 1200 cm while its search is busy) it SHALL run its search, anchored on the player, once a body tick. Past the band it SHALL plan a route to the player through the navigation grid on its planning turn and walk it. Combat, a flight or the scene taking the body SHALL stop the walk.

#### Scenario: The player walks away
- **WHEN** the player is 6 m from a standing follower that is not busy
- **THEN** the follower plans a route to the player and walks it

### Requirement: Searching the dead in the running game
The search's candidates in the running game SHALL be the level's dead bodies. Each SHALL be listed in its 600 cm room cell, with its worn items, its carried list and its hands. A take SHALL remove the item from the body's possessions and add it to the searcher's.

#### Scenario: A follower takes a dead body's sword
- **WHEN** a follower stands 1.5 m from the player, with a dead body 3 m away that wears a sword better than the one the follower carries
- **THEN** the follower gathers the body, looks it over, walks to it, kneels and takes the sword: the body's worn list no longer holds it and the follower carries it
