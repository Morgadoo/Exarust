## ADDED Requirements

### Requirement: Resolve drawn without multisampling
The system SHALL draw the supersampled scene's resolve to the window without hardware multisampling, so that the resolved scene shows in a window on every supported GPU.

#### Scenario: Supersampled window frame
- **WHEN** a level is drawn with supersampling on in a window
- **THEN** the display camera has one sample per pixel and the world shows under the interface
