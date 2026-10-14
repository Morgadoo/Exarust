# level-lighting Specification

## Purpose
Lights levels like the original: per-level ambient colour, sunlight and fog
from the level's environment record and the game's per-level presets.

## Requirements

### Requirement: Environment record
The system SHALL decode the environment record of a level file (ambient colours, vertical extent, sun settings) and report a malformed record as a level error without failing the level.

#### Scenario: Corpus
- **WHEN** `exa-tool corpus levels` runs over the installed game
- **THEN** every level reports an environment record and no section errors

### Requirement: Level lighting presets
The system SHALL light story levels and indoor arenas without a sun, light outdoor arenas with a shadow-casting sun and fog, and elsewhere add a sun only when the record asks for it or, in auto mode, when the level has no lights.

#### Scenario: Outdoor arena
- **WHEN** `exarust level arenao3` runs
- **THEN** the level is lit by a sun from the record's altitude and azimuth, with shadows and fog

#### Scenario: Story level
- **WHEN** a story level is shown
- **THEN** no sun is added and only its own lights and the ambient light it from the record
