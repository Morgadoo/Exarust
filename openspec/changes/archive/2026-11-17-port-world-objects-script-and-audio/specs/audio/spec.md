## ADDED Requirements

### Requirement: Finished effect voices
A force effect's end SHALL finish its voices: a playing one-shot SHALL play out unbound, a loop or a voice that is not playing SHALL be freed at once.

#### Scenario: A blast leaves while its release plays
- **WHEN** a blast leaves the scene while its release one-shot plays
- **THEN** the one-shot plays out and its handle is unbound

### Requirement: Story track volume
A story level's track SHALL play at the volume the story level hook's table gives it; other levels SHALL keep the level track's volume.

#### Scenario: The first and the sixth level
- **WHEN** exanima01 and exanima06 load
- **THEN** their tracks play at 0.1 and 0.25
