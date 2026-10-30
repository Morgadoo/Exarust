# Spec Delta

## ADDED Requirements

### Requirement: The target pass's turn
At the start of each tick the turn SHALL pass to the brain the last tick named, clearing the named brain and its wait, and the tick counter SHALL step. Each brain in the scene and the fight SHALL then ask for the turn: the brain on its turn SHALL stamp the counter as its last pass and run the target pass; any other SHALL be named for the next tick when its wait is strictly longer than the longest so far this tick. A brain out of the scene SHALL neither run the pass nor wait.

#### Scenario: Three brains
- **WHEN** three brains ask for the turn every tick from the first
- **THEN** none runs the pass on the first tick, and they run it in turn from the second, the first brain first

#### Scenario: A lone brain
- **WHEN** a single brain asks for the turn every tick
- **THEN** it runs the pass every other tick

#### Scenario: A dormant brain
- **WHEN** a dormant brain stands beside a present one for six ticks
- **THEN** it never perceives or runs the pass, and the present brain runs it on ticks 2, 4 and 6

### Requirement: The wait for the target pass
A brain's wait SHALL be the tick counter less its last pass, an unsigned difference of the wrapping 32-bit counter, and the whole counter for a brain that never had a pass.

#### Scenario: A newcomer
- **WHEN** a brain that never had a pass asks beside two that alternate
- **THEN** it runs the pass on the next tick after the current one, ahead of the brain that waited one tick

### Requirement: Between target passes
On a tick without its target pass a brain SHALL keep the last pass's report and the per-pass words its last perception wrote, its concern drifting toward the concern target that perception left, and SHALL neither enter nor leave combat.

#### Scenario: The hostile still engages
- **WHEN** a hostile and a neutral brain share the turn for 700 ticks
- **THEN** each has the pass every other tick and the hostile brain engages the player

## MODIFIED Requirements

### Requirement: Perception fills the brain's contacts
Once a body tick the brain on its target-pass turn SHALL clear its contacts' concern targets, closeness, view and noticed flag, then perceive, through the sense it holds, every character within the largest perception reach that the level's sight test lets it see and every character it already knows, and SHALL write the perception's sighting, closeness, attitude, threat and concern target into its contact for that character. A second frame on the same body tick SHALL perceive nothing.

#### Scenario: Visible and unseen
- **WHEN** the player stands two metres ahead, then is made unseen
- **THEN** the contact's closeness is 1 − 200/1200, then its sighting and closeness are 0

#### Scenario: Once a tick
- **WHEN** a contact's noticeability is cleared after a perception and the frame repeats on the same body tick, then the tick advances
- **THEN** it stays 0 on the same tick and is written again on the next

#### Scenario: One brain a tick
- **WHEN** two brains face the player and the turn rotates each tick
- **THEN** neither perceives on the first tick, one of them on the second and the other on the third

### Requirement: The AI tick
Once a body tick after the perception each brain SHALL decay its group alert, stop there when its body is out of the fight (condition bit `0x800`), and otherwise drift its morale and integrate its contacts; on its target-pass turn it SHALL also run the target pass over every character seen this tick, each contact's way out answered from the brain's room, exits and fled rooms. It SHALL keep the report and copy its concern and hostility to the combatant's contacts.

#### Scenario: One pass a tick
- **WHEN** a brain holds a seen contact with hostility 5 for the player
- **THEN** one tick's pass enters combat at level 3 and attends the player, and a cooled contact the next tick leaves it at level 0
