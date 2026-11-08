## ADDED Requirements

### Requirement: Lockstep runs visit entities in one order
In lockstep, the simulation SHALL apply each bridged component's values and removals from the frame in entity order, so that its queries visit entities in the same order in every run of a scene, whatever order the frame's own archetypes give them.

#### Scenario: Four identical scenes
- **WHEN** four identical `--fixed-frames --trace-all` scenes of exanima01 run in one grid
- **THEN** their traces are byte-identical, rows of each tick in the same order

#### Scenario: Frame-only markers
- **WHEN** some of the frame's entities carry a component the bridge does not carry, putting them after the others in the frame's queries, and then each gets a bridged component in the same frame
- **THEN** the simulation's queries visit them in the same order as without the marker
