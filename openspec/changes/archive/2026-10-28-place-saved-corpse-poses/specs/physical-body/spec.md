# Spec Delta

## ADDED Requirements

### Requirement: Placed corpses
A level's dead character SHALL be spawned with every body part on the world frame its placement saved (bone tree first, flat frames otherwise, else the built pose), the character frame at the restored root's world origin, no muscles and no sampled motion, and the placement's position and yaw unused. Its body SHALL be registered with zero velocity and countdown 0, so it sleeps in its first tick's post-step and is in deep sleep from the next, and SHALL wake only as any sleeper does.

#### Scenario: Registered at rest
- **WHEN** a placed corpse's body is built and ticked twice
- **THEN** its countdown is 0, it is asleep after the first tick and in deep sleep after the second

#### Scenario: Shipped corpses
- **WHEN** the seven `exanima01` corpses are placed
- **THEN** every part sits within 2 cm of its saved frame and each body is asleep after one tick

### Requirement: Woken sleepers take the post-step
A sleeper an awake body's contact wakes SHALL take that tick's post-step, as the original's solver list does, so one woken to a countdown under 2 falls asleep again in the same tick and its next tick removes the push's motion.

#### Scenario: Touched by a placed corpse
- **WHEN** a deep sleeper overlaps a body registered at rest by 2 cm
- **THEN** after one tick both are asleep, after two both are in deep sleep, and neither moves afterwards
