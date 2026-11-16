## ADDED Requirements

### Requirement: Save exporter switches
Every save export of a level section SHALL use the save exporter's switches: save mode on and the triangle-data bit set, so no mesh in the section stores its triangle edge or flag sub-chunks.

#### Scenario: A shipped level saved
- **WHEN** exanima01's section is written for a save
- **THEN** it reads back with no mesh holding triangle flags or edges and is smaller than the shipped section

### Requirement: Saved props
A save SHALL write each prop the scene holds at its live placement in its parent's frame, a prop over a moved free body taking that body's motion since it was registered; a prop within a millimetre of its record keeps the record.

#### Scenario: A pushed prop
- **WHEN** a prop root stands 40 cm from its record and the game saves
- **THEN** the section's prop record is 40 cm from the shipped one and a prop placed under it keeps its world place

### Requirement: Hung bodies in a save
A body hung from the world by its links (a door's leaf, a lever's arm) SHALL move neither the saved placement of the object nor of the prop it hangs in.

#### Scenario: A swung door
- **WHEN** a door's leaf has swung open and the game saves
- **THEN** the door's placed object keeps its placement
