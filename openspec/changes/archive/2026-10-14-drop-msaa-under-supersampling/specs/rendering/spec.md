## ADDED Requirements

### Requirement: No multisampling under supersampling
The system SHALL draw the supersampled scene without hardware multisampling, as the original does (`HWMultisample = 0`); the sub-frames are the only antialiasing.

#### Scenario: Supersampled frame
- **WHEN** a level is drawn with supersampling on
- **THEN** the scene image has one sample per pixel
