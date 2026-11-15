## ADDED Requirements

### Requirement: The alert look turns to the body's target
The alert look-around SHALL take the yaw from the body to the body's target as its foe when the body has a target, taking that yaw as its look past 45 degrees, and SHALL pan only when the body has no target.

#### Scenario: A marked brain with a target to its side
- **WHEN** an out-of-combat brain with panic 0.5, the in-combat mark and a body target 5 m along +x runs its AI tick
- **THEN** its look yaw is the yaw toward +x

### Requirement: The nearby gather walks the room and its exits
The AI tick SHALL gather the nearby list from the characters of the brain's level room, then of the room behind each of its exits in turn, an exit leading to the same room as the exit before it skipped; each candidate SHALL pass the sight test, the room-to-room light reach. A brain in no room SHALL keep its list.

#### Scenario: Two characters in the next room
- **WHEN** a brain's room has an open door to a room holding one character within 2 m of the door point and one far from it
- **THEN** the nearby list holds the first and not the second

### Requirement: The closing walk
A follower inside its personal stop that replans SHALL plan a forced walk to the closing goal: the leader's aim point, moved, when the follower is within 200 cm of the leader, to 400 cm from it along the leader-to-follower line, with motion 0x14 and the closing mode, and a 300 cm radius when the goal lies over 50 cm away across the ground.

#### Scenario: A follower 50 cm from the player
- **WHEN** a follower stands 50 cm along +x from the player and replans inside its personal stop
- **THEN** its walk goes to 400 cm along +x from the player in the closing mode

### Requirement: The closing mode ends with its walk
The closing mode SHALL end when its walk ends, so the follow returns to its band and pursuit.

#### Scenario: A follower whose closing walk ended
- **WHEN** a closing follower's walk has ended and the player is 4 m away
- **THEN** it pursues the player again
