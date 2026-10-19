# Spec Delta

## ADDED Requirements

### Requirement: Dropped weapon enters the solver
A collapsing body's dying tick SHALL put a dropped weapon that has a solid shape into the body solver, awake at 256, with its own mobility, friction 0.75, its spheres, and the velocity it had. One solver tick moves it. A weapon with no solid shape spawns nothing. A second dying tick adds no body.

#### Scenario: A solid weapon falls free
- **WHEN** a collapsing body holds a solid weapon of mobility 1.25 at height 1 with velocity 1.5 and one sphere, and a flat weapon, the dying tick runs twice, and the free body is stepped once against no level
- **THEN** the solid weapon is awake at 256 with friction bits 0x3f400000, mobility bits 0x3fa00000 and that sphere, the flat weapon spawns nothing, the second tick adds no body, and the first particle is at bits 0xbe3a7bb7, 0x3f7fa2b5 and 0xbe4ccccd
