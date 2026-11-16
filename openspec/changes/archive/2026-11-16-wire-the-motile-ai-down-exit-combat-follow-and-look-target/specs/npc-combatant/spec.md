## ADDED Requirements

### Requirement: The down exit's arc
While the down exit runs, the fighter SHALL publish the attack arc kind "recovering", or "none" when its condition is exactly the getting-up word and its clip is past 0.9 (a double compare; not a number is not past); the position scores of the fighters around it SHALL read that kind in place of the arc its clip would publish, and once it is up its arc SHALL be its body's again.

#### Scenario: Down, getting up, up
- **WHEN** a fighter in combat falls, then lies in the getting-up word with its clip at its start, then stands
- **THEN** its published kind is recovering twice, and then none is published from the down exit

### Requirement: The down exit's stance and pose
The down exit SHALL write the guard as the body's stance without starting it, so the clip under way ends in the guard; it SHALL clear the body's pose word, and so SHALL every combat tick that goes on past the follow and the leave, which ends a statue's rest.

#### Scenario: Falling in the combat idle
- **WHEN** a fighter whose stance is the combat idle falls in combat
- **THEN** its stance is the guard, its fall plays on and ends in the guard, and its pose word is cleared

#### Scenario: A resting statue fights
- **WHEN** a statue at rest is taken into combat and its combat tick runs
- **THEN** its rest ends
