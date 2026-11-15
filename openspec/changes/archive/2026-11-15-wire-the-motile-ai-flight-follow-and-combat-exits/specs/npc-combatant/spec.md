## ADDED Requirements

### Requirement: The leave restores the idle state
The target pass's leave of combat SHALL restore the idle state as the current state at its first step, as the combat state's own leave does.

#### Scenario: A calm pass while fighting
- **WHEN** a fighting brain's target pass finds its contact calm and leaves combat while its roam walked home
- **THEN** its routine is back at the first wait

### Requirement: The foe's footprint after every exit
The combat state SHALL stamp the footprint of the foe it lifted back on the grid however the state ends, the early exits (down, leave, chase, charge) included.

#### Scenario: A tick that ends at the chase
- **WHEN** a stuck fighter's combat tick ends at the chase
- **THEN** its foe's footprint is on the grid as before the tick

### Requirement: The down exit while down
While the body's condition carries the down bit and the combat state is current, each tick SHALL take the down exit: the aim, the heading and the brain's look, heading and head look take the yaw of the body's facing, and the body's look directions the facing.

#### Scenario: A knocked-down fighter turned by its fall
- **WHEN** a fighting body is down and faces yaw 1.25
- **THEN** its aim, heading and look are the yaw of that facing

### Requirement: The cast flag times out in the AI tick
The AI tick SHALL let go of the cast chooser's casting flag 500 ms after the cast began, before its condition gate, unless the body's condition reads casting; the compare SHALL be widened so it cannot wrap.

#### Scenario: A cast the body never started
- **WHEN** an out-of-combat brain's casting flag was set at 0 ms and the AI tick runs at 600 ms with the body not casting
- **THEN** the flag is clear, and a casting body's flag is kept
