## ADDED Requirements

### Requirement: Dialogue keys
While a dialogue panel is open the digits 1 to 9 SHALL pick reply 0 to 8 and the Talk key SHALL end every conversation the player is in; otherwise the Talk key SHALL start talking to the focused character, and send nothing when none is focused.

#### Scenario: Picking and leaving
- **WHEN** the panel is open and the player presses 1, then the Talk key
- **THEN** reply 0 is picked, then the player's conversations end

### Requirement: Reply panel sync
The reply panel SHALL be rebuilt from the engine's entries only when they or the narrator flag change: each entry measured at the panel width less eight units, the `[Other]` toggle (topic 0xFFFF) appended with zero lines, then laid out; a closed or empty engine panel SHALL close and clear it.

#### Scenario: An unchanged list keeps its animation
- **WHEN** a one-entry panel is synced at 1000 ms and again unchanged at 1200 ms
- **THEN** the height animation still ends at 1500 ms

### Requirement: Script cast mirror
Each frame every live character SHALL reach the script cast with its name, roles, health as the red pool over the maximum, arousal (0 without a brain) and the collapsing/dead state bit, keeping its relations, topic memory and role values; a role list a script changed SHALL be written back to the AI's roles first and its change flag cleared.

#### Scenario: A script adds a role
- **WHEN** a character holds role 5 and a script adds role 7 in front of it
- **THEN** after the next frames both the AI and the cast hold roles 7 then 5

#### Scenario: Script health
- **WHEN** a body has red health 0.2, yellow 0.1 and maximum 0.25
- **THEN** the cast health is (0.2, 0.25), so `Health` gives 80
