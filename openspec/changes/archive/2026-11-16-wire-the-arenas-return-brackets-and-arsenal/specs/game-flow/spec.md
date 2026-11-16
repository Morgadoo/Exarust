## ADDED Requirements

### Requirement: The arena's return holds the screen over the hub's load
The arena's return to the hub SHALL be a staging: its callback SHALL raise the pause counter when it is zero, stop the music with the fade on, end with the playing phase and count as a level change, so the screen stays black and its frames wait until the change is done.

#### Scenario: A loading return
- **WHEN** a return's transition reaches black and five loading frames follow, then three frames without the load
- **THEN** it raises the pause and stops the music at black, stays black and lowers nothing while loading, and lowers the pause on the third frame after
