# Spec Delta

## ADDED Requirements

### Requirement: Placed model-less records
A level's placed record without a model name SHALL spawn its container volume node when it has a volume section, else its operative placeholder, and either SHALL carry the record's sections, its mechanism records and, for a teleporter or reconstruction device, its device.

#### Scenario: A placed volume
- **WHEN** `exanima01` is spawned
- **THEN** each model-less record with a volume section has a container volume node
