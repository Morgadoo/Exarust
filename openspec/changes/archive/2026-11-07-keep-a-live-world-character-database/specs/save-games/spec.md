## ADDED Requirements

### Requirement: Live world character database
The running game SHALL keep a world character database: a new story SHALL hold the player's slot under id 1, a story load SHALL import the save's `#worldchrdb`, and a story save SHALL write the store's records beside the player.

#### Scenario: A save loaded back
- **WHEN** a story save whose `#worldchrdb` holds the player and a follower is loaded
- **THEN** the live store holds both records

### Requirement: Followers registered in the world character database
A follower taking a door SHALL be registered in the world character database: a record that is not the world store's SHALL keep its origin (the level it leaves and its local id) and join the store under its first free id with the world tag, and every registered record SHALL take the level it leaves as its level. On arrival its record SHALL take the destination's id as its level and keep its world id.

#### Scenario: Derrin registered at a door
- **WHEN** Derrin (record 0x2b) follows the player from exanima01 (level 2) to exanima02 (level 3) in a new story and the game is saved there
- **THEN** his id becomes 0x40000002 with origin (2, 0x8000002b), and the save's `#worldchrdb` holds the player under 0x40000001 and Derrin under 0x40000002 with level 3
