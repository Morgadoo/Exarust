## ADDED Requirements

### Requirement: Root chunk loop
The scene's and the database-only root loops SHALL read another chunk only while more than 8 bytes remain.

#### Scenario: A trailing empty chunk
- **WHEN** a scene ends in an 8-byte chunk header with no body
- **THEN** the chunk is not read
