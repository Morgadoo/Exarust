# Spec Delta

## MODIFIED Requirements

### Requirement: Takes carried out
A take SHALL despawn a loose item, remove a container's first matching entry, release an item from the corpse's grip (the second hand slot and its carried entry) or else remove it from the worn list, or remove a carried entry and clear the fourth hand slot when it holds the same item. Wearing SHALL put the item on and out of the carried list before taking it from its source.

#### Scenario: Each source
- **WHEN** a container, a corpse wearing one item and gripping a carried one, and a loose item are taken from
- **THEN** each take edits its source, and the gripped item leaves the second hand slot and the carried list but not the worn list

#### Scenario: A carried take clears the fourth hand
- **WHEN** a carried item that the fourth hand slot also holds is taken
- **THEN** its entry goes and the fourth hand slot is empty

## ADDED Requirements

### Requirement: Phrase tables
The comment templates SHALL be the original's: for one item "I found [a].", "There's [a] here.", "Found [a] here.", "Got [a] here.", "[a] here, if you want it." and the never-drawn "This [i] could be useful."; for a pair the same with "some [i]" ("Some [i] here, if you want them.", "These [i] could be useful."); for a further item "And [a].", "Found [a] too.", "Also found [a].", "There's [a] too.", and for a further pair the same with "some [i]".

#### Scenario: A first helmet
- **WHEN** the first comment names a helmet and draws the first template
- **THEN** it reads "I found a helmet."

### Requirement: Approach answer
While approaching, its own pack open on screen SHALL answer not busy; a walking attention that takes the tick SHALL leave the answer busy and the state unchanged.

#### Scenario: The walk takes the tick
- **WHEN** the approach's attention of 5.1 takes the tick
- **THEN** the tick answers busy and the search stays approaching with no try counted
