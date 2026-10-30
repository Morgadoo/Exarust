# Spec Delta

## ADDED Requirements

### Requirement: Volume routine
A volume section's body SHALL keep, after its fixed words, a routine: its length at `0x14` and its text from `0x18`; a length of 0 or past the body's end SHALL give none. A volume with class `0x100` and low byte `0x10` is a script zone whose id is its entrance word. Every shipped level holds: eleven enter script zones across the story levels, six of them in `exanima01`, whose `Checkpoint` zone (half extents 80, 123, 50) runs `CheckpointSave(1);`.

#### Scenario: The checkpoint zone
- **WHEN** a volume body of flags `0x1110`, entrance 4 and an 18-byte routine `CheckpointSave(1);` is read
- **THEN** it is a script zone with id 4 and that routine
