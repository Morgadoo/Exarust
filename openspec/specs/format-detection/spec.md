# format-detection Specification

## Purpose

Identifies the format of any pack entry from its leading bytes, because entries
in the main packs carry no file extension.

## Requirements

### Requirement: Detection by magic
The system SHALL classify an entry as pack, chunk tree, image, terrain, terrain table, database, skeleton/motions, power, weapon factory, actor, WAVE audio, DDS, or unknown, from its first four bytes.

#### Scenario: Retail corpus
- **WHEN** every entry of every retail pack is classified
- **THEN** only the dialogue file and the string library in `Resource.rpk` are reported as unknown

#### Scenario: Too short to classify
- **WHEN** an entry is shorter than four bytes
- **THEN** it is reported as unknown without error

### Requirement: Extension for extraction
The system SHALL provide a file extension for each detected format so extracted entries can be opened by other tools.

#### Scenario: Extracting sounds
- **WHEN** entries of `Sound.rpk` are extracted
- **THEN** each file is written with a `.wav` extension
