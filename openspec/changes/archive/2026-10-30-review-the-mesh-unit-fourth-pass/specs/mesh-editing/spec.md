## ADDED Requirements

### Requirement: Colour channel creation
Adding a colour channel SHALL append a stream with the first stream's id or the shared-channel bit, the colour format word, the first stream's channel and element count, and opaque white bytes unless bytes are supplied.

#### Scenario: A square
- **WHEN** a colour channel is added to a four-corner square without bytes
- **THEN** the new last stream has the colour id and format, channel 0 and four `[255, 255, 255, 255]` entries
