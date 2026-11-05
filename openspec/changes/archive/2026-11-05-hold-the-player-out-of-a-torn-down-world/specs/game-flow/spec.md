# Spec Delta

## ADDED Requirements

### Requirement: Player out of a torn-down world
A teardown with no level after it SHALL take the surviving player's body out of the scene. The body SHALL leave the physics solver and the character ticks, and SHALL be hidden with its root, so it cannot fall or die in the emptied world. Once a level is in again and its loading screen has gone, the body SHALL come back into the scene. A body the fall took out of the scene SHALL stay out.

#### Scenario: The creator over a running level
- **WHEN** the practice creator opens over a running `exanima01` and stays open for a minute
- **THEN** the player is not lost and no death fade starts, and the creator's form and preview stay drawn

#### Scenario: A level after the teardown
- **WHEN** a plain teardown is followed by a level load
- **THEN** the player's body stays out of the scene while the loading screen shows, and comes back once it has gone
