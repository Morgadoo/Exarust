# Spec Delta

## ADDED Requirements

### Requirement: Init assignments
Applying roles SHALL start from Activity 1 and IdleActivity 0 and run each role's init assignments newest role first, so an older role wins. A value SHALL be an integer, a `$hex` integer or a role variable (0 when the held role stores none). Any other statement SHALL be reported unrun.

#### Scenario: Older wins
- **WHEN** Activity Wait is newer than Activity Patrol, and a role's script adds a concern
- **THEN** the character patrols, and the concern statement is reported unrun

### Requirement: Activity to idle state
An active controlling effect SHALL force Activity 4 unless resisted. Activity 1 SHALL roam, 2 wait, 3 sleep, 4 follow (joining the party with a 400 cm band), 5 stand sentry and 7 patrol; 0, 6 and any other SHALL wait. A follower that no longer follows SHALL leave the party.

#### Scenario: Control
- **WHEN** a patrolling character carries a controlling effect, with and without resistance, having followed before
- **THEN** it follows without resistance, and with it patrols and leaves the party

### Requirement: Idle-activity registry
The idle activity SHALL be created from the registry when there is none and replaced when its id changed. The registry SHALL refuse a taken id and hold the search as 6.

#### Scenario: The explorer
- **WHEN** an explorer's Activity variable is 4 with IdleActivity 6, and it later holds no such role
- **THEN** it follows with a 400 cm band and gets the search, which then goes

### Requirement: The wait and the home-side point
The wait SHALL stand after the disturbance and alert checks. The home-side point SHALL be the exit of the current room toward the room the last wander point aimed at, at height 0, mode 3, stop 20 cm and radius 150 cm, ignoring height; no room or no such exit SHALL pick nothing.

#### Scenario: Back to the doorway
- **WHEN** a roam in a room with exits toward rooms 4 and 9 last wandered toward room 9, and a waiting character hears a disturbance of 0.31
- **THEN** the home-side point is the exit toward room 9 with mode 3, and the waiting character walks to the noise
