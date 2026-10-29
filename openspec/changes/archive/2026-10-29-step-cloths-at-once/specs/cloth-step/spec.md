## ADDED Requirements

### Requirement: Cloth mesh write-back
The system SHALL write a cloth's particle positions into its mesh once a frame, after the frame's fixed ticks, and only when the cloth stepped since its last write; the fixed ticks SHALL leave the mesh unchanged.

#### Scenario: Four ticks in one frame
- **WHEN** an awake hair cloth steps four ticks in a frame
- **THEN** its mesh keeps its old positions through the ticks, and after the frame's write-back holds each vertex's particle position in metres

### Requirement: Cloths step independently
The cloth stage SHALL leave each cloth exactly where stepping it alone would leave it, whatever the other cloths of the level, while the object list's re-buckets keep the list's order.

#### Scenario: Fifteen hairs
- **WHEN** fifteen hair cloths hang from fifteen living bodies among 336 body parts for 40 ticks
- **THEN** each hair's particles are bit for bit those of the same hair stepped with the other fourteen removed
