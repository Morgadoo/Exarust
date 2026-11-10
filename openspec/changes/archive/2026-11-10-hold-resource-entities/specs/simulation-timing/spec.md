## ADDED Requirements

### Requirement: Lockstep runs number their entities alike
Two `--fixed-frames` runs of one build and scene SHALL give the simulated bodies the same entity ids. A resource is an entity spawned when first inserted, so the frame's world SHALL spawn an empty entity for every registered resource type at one fixed moment (the start of its second update), and the simulation's world one for each bridged resource when the bridge starts; a resource first inserted later (when a load finishes, or in whichever step its value crosses) lands on that entity. Under `--fixed-frames` the startup schedules SHALL run their systems one at a time, since a spawn takes its id when it is queued.

#### Scenario: The arena duel
- **WHEN** four identical `play arena --opponent --press D@200-260,W@400-460,S@600-700 --frames 3000 --fixed-frames --trace-all` scenes run
- **THEN** their traces are byte-identical, the `body` column included

#### Scenario: A resource inserted after a spawn
- **WHEN** one world inserts a held resource before spawning an entity and another inserts it after
- **THEN** the spawned entity has the same id in both
