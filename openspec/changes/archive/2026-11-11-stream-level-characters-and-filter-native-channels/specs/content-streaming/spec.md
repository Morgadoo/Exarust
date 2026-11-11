## ADDED Requirements

### Requirement: Level characters on the stream thread
While a level's collision builds, the running game SHALL hand each of the level's character bodies (the body model morphed by the record's physique, once per model variant not yet built) to the content stream's streamed pass, which SHALL build them on the stream thread only when no job or request is queued, nearest the player's start first.

#### Scenario: A level with characters
- **WHEN** exanima01 loads with the stream on
- **THEN** its 35 body variants are built on the stream thread before the spawn, and the spawn builds none of them

### Requirement: Streamed bodies handed over
The main-thread pump SHALL hand over each streamed body as it is built, and the play loader SHALL add it to the model cache in that frame.

#### Scenario: Bodies arrive while the collision builds
- **WHEN** a streamed body is built during the collision build
- **THEN** it is in the model cache before the spawn, under the key the spawn asks for

### Requirement: The spawn does not wait for streamed bodies
The level's spawn SHALL not wait for the streamed pass: it SHALL drop the streamed characters not yet started and build what is missing itself, in the same frame and order as without the stream.

#### Scenario: A short collision build
- **WHEN** the collision is built before every streamed body
- **THEN** the spawn happens in the same frame, builds the rest itself, and the lockstep trace's rows are those of `--sync-content`
