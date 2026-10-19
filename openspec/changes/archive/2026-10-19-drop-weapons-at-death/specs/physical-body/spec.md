# Spec Delta

## ADDED Requirements

### Requirement: Weapons dropped at death
A collapsing body's dying tick SHALL drop every held weapon when it has not already. Each keeps its velocity, takes its own mobility, friction 0.75 and a drag of that mobility times 0.175, and clears its striker flags. The hands, the class word and the primary SHALL be empty. A second tick SHALL add nothing. An empty pair SHALL stay empty.

#### Scenario: Two weapons leave the hands and stay loose
- **WHEN** a collapsing body holds two weapons of mobility 1.25, the first with velocity 1.5, and its dying tick runs twice, and an empty pair with a class word is released at the collapse start
- **THEN** the first tick leaves both loose with drag bits 0x3e600000, friction bits 0x3f400000, no striker flags and the velocity 1.5, the hands and the class word and the primary are empty, the second tick adds nothing, and the empty pair stays empty
