# Spec Delta

## MODIFIED Requirements

### Requirement: The duellist engages
A level character's duellist SHALL take the guard when its brain's target pass enters combat against the duellist's target, and SHALL leave the fight when a pass calms down while it fights. The mind's own rating SHALL NOT start the fight.

#### Scenario: Hostile contact
- **WHEN** a brain whose roles hate the player sees the player two metres away for 700 frames
- **THEN** its target pass enters combat and its duellist is engaged

#### Scenario: Neutral role
- **WHEN** a brain with no hostile role sees the player as long
- **THEN** its pass stays at level 0 and its duellist waits

## ADDED Requirements

### Requirement: Perception fills the brain's contacts
Every frame each brain SHALL clear its contacts' concern targets, closeness, view and noticed flag, then perceive every character within the largest perception reach and every character it already knows, and SHALL write the perception's sighting, closeness, attitude, threat and concern target into its contact for that character.

#### Scenario: Visible and unseen
- **WHEN** the player stands two metres ahead, then is made unseen
- **THEN** the contact's closeness is 1 − 200/1200, then its sighting and closeness are 0

### Requirement: Dead, knocked-out and lost characters
A dead, asleep or removed character's concern and hostility SHALL be cleared on the brain; only a knocked-out character SHALL be skipped by the calming. A tracked target that moves out of reach SHALL raise the brain's alert to the lost floor.

#### Scenario: Dead and downed
- **WHEN** a contact with concern 3 and hostility 5 is dead, or only down
- **THEN** both are cleared for the dead one and kept for the downed one

#### Scenario: Lost target
- **WHEN** a tracked target moves 25 m away
- **THEN** the contact is no longer tracked and the alert is 40

### Requirement: The AI tick
Every frame after the perception each brain SHALL drift its morale, integrate its contacts and run the target pass over every character seen this tick, keep the report on the brain, and copy its concern and hostility to the combatant's contacts.

#### Scenario: One pass a tick
- **WHEN** a brain holds a seen contact with hostility 5 for the player
- **THEN** one tick's pass enters combat at level 3 and attends the player, and a cooled contact the next tick leaves it at level 0

### Requirement: Calming from the perceived attitude
The integration SHALL raise a seen contact's hostility by `(1 − decay)·attitude·(1 − restraint)·5e-5·scale` subtracted, only when the perceived attitude is below zero.

#### Scenario: Hostile and friendly
- **WHEN** a seen contact's attitude is −20, then +20
- **THEN** its hostility rises above its 0.999 decay, then only decays
