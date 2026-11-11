## ADDED Requirements

### Requirement: The AI reads body directions through the engine
The AI's look, turn, snap, settle, leave and search routines SHALL take the yaw of a body direction by the engine's fast heading, build a direction from a yaw by its fast sine and cosine, and take the idle look's yaw by its fast two-argument arctangent of the look's x and z (the heading while a look target still runs).

#### Scenario: A body facing one radian
- **WHEN** the AI reads a body facing along yaw 1.0
- **THEN** it reads the engine's heading of that direction, within 5e-4 of 1.0, and a direction it builds from a yaw equals the engine's direction of that yaw

### Requirement: A standing body keeps its facing
The stand-in that turns an NPC's model to the AI's eased yaw SHALL remember that yaw, and the facing the AI reads SHALL be that yaw while the model keeps the rotation the stand-in wrote, and the engine's heading of the model's forward once anything else moved it.

#### Scenario: An idle brain stands for 200 ticks
- **WHEN** an idle brain placed facing yaw 1.0, with no sway, ticks 201 times
- **THEN** its model faces the engine's heading of the placed facing after the first tick and the same yaw, within 1e-6, after the last

## MODIFIED Requirements

### Requirement: Look writes on leaving and settling
Leaving combat SHALL set the look, the heading and the eased offset to the body's look yaw and keep the head look, its target and the sway timer. The settle after a room walk SHALL set the look, the heading and the head look to the yaw of the body's heading direction, keep the eased offset and draw the next sway 1–4 s out. The aim snap SHALL copy the body's facing vector into the body's look, look smoothing and heading directions, and take its heading as the yaws.

#### Scenario: Leaving combat
- **WHEN** combat is left with the body's look yaw 0.75
- **THEN** the look, heading and offset are 0.75 and the head look and the sway timer are unchanged

#### Scenario: The snap copies the facing
- **WHEN** the aim snap takes a body facing along yaw −0.5
- **THEN** the body's look, smoothing and heading directions are that facing vector and the AI's yaws its heading
