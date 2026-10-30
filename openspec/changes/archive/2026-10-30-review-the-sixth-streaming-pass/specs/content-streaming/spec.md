## MODIFIED Requirements

### Requirement: Player focus
The arrival variable SHALL give the operative kind that names the player: one kind for an arrival, another for a start or load, none otherwise. A placed object of the operative node kind SHALL be the player when its operative kind equals that kind. A placed object of the volume node kind SHALL be the player when its marker's entrance equals the arrival variable and its previous level equals the level the session came from (all ones for none). No other object SHALL be the player.

#### Scenario: An actor by kind
- **WHEN** a level begins on an arrival and holds actors of operative kind 15 and 18
- **THEN** the actor of kind 15 is the player

#### Scenario: A marker by arrival
- **WHEN** a marker's words are the arrival variable and the previous level's id
- **THEN** it is the player, and with any other previous level it is not

#### Scenario: The node kind decides first
- **WHEN** an object of operative kind 15 has a model and a door section, or a marker's record has a model
- **THEN** on an arrival neither is the player

## ADDED Requirements

### Requirement: Placed object node kind
The player test SHALL read the node kind the database builder gives a placed object: on a model tree the door kind when the record has a door section, else the operative kind when it has an operative section, and none for a weapon or an item body; the volume kind for a model-less record with a volume section; the operative kind for a model-less operative record's placeholder.

#### Scenario: A door before an operative
- **WHEN** a record with a model holds a door and an operative section
- **THEN** its node kind is the door kind, and without the model and with a volume section it is the volume kind
