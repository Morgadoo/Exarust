## ADDED Requirements

### Requirement: The goal entity
The search's pick SHALL link the brain's goal entity (made at the first pick, weight 5, under an id that names no character) to the picked object and set the brain's pick flag; a give-up and the loot's end SHALL clear the flag and unlink the entity, which the brain keeps.

#### Scenario: Picked and let go
- **WHEN** a search picks a chest 3 m away and later gives it up
- **THEN** the goal entity is linked to the chest with the flag set, then unlinked with the flag clear

### Requirement: The goal in the target pass
While the pick flag holds the target pass SHALL weigh the goal entity first: its object contact clears its alarm and, linked within 1200 cm, takes the weight as its noticed and sighting levels with no attention. With nothing that outranks it the pass SHALL attend it; a calm pass SHALL keep attending it after the flag clears.

#### Scenario: A calm brain picks
- **WHEN** a brain with no other contact picks an object 3 m away and runs its pass
- **THEN** it attends the goal entity, whose object contact has sighting 5 and attention 0

### Requirement: The goal's attention stays
The integration SHALL never raise the goal entity's attention, and a pass that notices a character above it SHALL attend that character.

#### Scenario: Twenty passes and a foe
- **WHEN** a brain attends its goal for 20 passes, then sees a concerned character
- **THEN** the goal's attention stays 0 and the pass attends the character

### Requirement: Waiting for the goal
The search's look, path, settle and loot SHALL read the brain's attention: on its goal entity, on someone else, or on no one. The idle attention's window SHALL be 50° on the goal under the pick flag, and the walking head SHALL look at the goal's object.

#### Scenario: The live search
- **WHEN** a follower's search picks a chest under the real AI tick
- **THEN** it walks to the chest only once its target pass attends the goal entity linked to the chest
