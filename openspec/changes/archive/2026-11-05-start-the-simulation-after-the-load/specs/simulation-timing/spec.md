## ADDED Requirements

### Requirement: No fixed step behind the loading screen
While the loading screen shows, the fixed step SHALL not run and the time the frames bring SHALL not be kept for it, so the first tick after a load comes with the first frame after it, however many frames the load took.

#### Scenario: Loads of different lengths
- **WHEN** one run's load takes 3 frames and another's 47, and both then run 20 frames
- **THEN** no tick runs during either load and both runs tick alike after it

### Requirement: Static colliders in hand-out order
A level's static colliders SHALL join the physics world in the order their builds were handed out, whatever order the builds finish in, and a frame SHALL hand out builds by a triangle budget, not by its clock.

#### Scenario: Two scripted runs
- **WHEN** a scripted scene (exanima01's walk, exanima06's W, D and S script, the arena duel) runs twice with `--fixed-frames`
- **THEN** both runs' body traces agree from their first tick
