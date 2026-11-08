## MODIFIED Requirements

### Requirement: Saved level section
Every save SHALL write the current level's section under its locale name after the world databases, holding the live local tier and its library and the placed objects as the scene holds them: a spawned object at its live placement with its live id and condition, a spawned object that is gone left out with the objects inside it, an unspawned one as loaded, and an object dropped since added at the end. A story save SHALL also write the scene's characters: each stored placement whose character stands in the scene takes its live place, heading and death when it moved or died, one whose record left the level's character database is left out, one with no character is kept, and a scene character with no placement is added with its record's id; the section's character database SHALL be the live local one.

#### Scenario: A taken and a dropped object
- **WHEN** a spawned object holding another is gone and an item was dropped into the world, and the game saves
- **THEN** the section's list leaves both out, closes up the later container numbers and ends with the dropped item

#### Scenario: A carried follower saved and loaded
- **WHEN** Derrin follows the player from exanima01 to exanima02, the story is saved there and the save is loaded
- **THEN** the save's exanima02 section places 0x40000002 beside the level's own characters and the load places Derrin from the world character database

## ADDED Requirements

### Requirement: Live local character database
The running game SHALL keep the level's own character database: it SHALL be read from the level's section at each load, a follower registered at a door SHALL be deleted from it, and a story save SHALL store each scene character's live record into it before writing it.

#### Scenario: Derrin leaves exanima01
- **WHEN** Derrin (record 0x2b) is registered in the world character database at a door
- **THEN** record 0x2b is no longer in the level's character database

### Requirement: Scene characters flushed by a story save
Before writing its sections a story save SHALL store every scene character's live possessions and health into its record, in the world character database for a world record and in the level's for the rest, and SHALL put the live player's record over the world database's player slot.

#### Scenario: A save after a door
- **WHEN** the story is saved in exanima02 with Derrin following
- **THEN** the world character database holds the live player under id 1 and Derrin's live record under 0x40000002
