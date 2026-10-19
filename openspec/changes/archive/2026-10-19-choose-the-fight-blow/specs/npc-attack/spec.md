# Spec Delta

## Purpose
The blow an NPC picks once a fight has started: overhead, thrust, or a side.

## ADDED Requirements

### Requirement: Fight blow
An unarmed fighter inside eight tenths of its reach SHALL be able to swing overhead, and outside that reach SHALL be able to take the other side. An armed fighter stepping forward slowly SHALL thrust when its preference clears the draw, and SHALL take the overhead when its age is at least a half. A facing score above 4 SHALL take the other side. A quadruped beyond eight tenths of reach SHALL be able to thrust. Energy under eight tenths of the maximum SHALL refuse the swing.

#### Scenario: Reach, preference and energy
- **WHEN** an unarmed fighter at full energy is inside eight tenths of reach and the generator starts at 0, then the same fighter is outside that reach, then an armed fighter stepping forward with preference 9, then that fighter at age one half, then a fighter whose attack phase is 100, then a quadruped beyond eight tenths of reach, and the outside unarmed fighter is tried again at 0.79 of its energy
- **THEN** the blows are overhead, the other side, thrust, overhead, the other side and thrust, and the last try starts nothing
