# npc-steering Specification

## Purpose
How an NPC follows a path: the heading and desired velocity it writes, the same fields the player writes.

## Requirements

### Requirement: Path order
Following a path SHALL write a heading and a desired velocity. Along a straight path the heading already faces, the velocity SHALL be the player's walk speed, or the player's jog speed when the urgency is at least 1. A path of one point SHALL leave the velocity at zero.

#### Scenario: Straight walk
- **WHEN** the path runs 8 m forward and the heading already faces it
- **THEN** the velocity is 1.4 m/s along that heading

#### Scenario: Urgent
- **WHEN** the same path is followed with urgency 1
- **THEN** the velocity is 3.6 m/s along that heading

### Requirement: Duellist uses the order
A duellist closing on its target SHALL set its body's desired velocity to the order for the straight path from itself to the target.

#### Scenario: Closing in
- **WHEN** a duellist 8 m from its target has entered combat
- **THEN** its desired velocity equals that order and it plays the combat walk
