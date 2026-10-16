## MODIFIED Requirements

### Requirement: Walk to the cursor
The system SHALL walk the player toward the cursor's point on the plane 80 cm above the player's ground while the right mouse button is held, running when that point is more than 3 m away. The point SHALL be found through the scene camera's own viewport, also when the scene is drawn supersampled into a larger image.

#### Scenario: Far cursor
- **WHEN** the right button is held over a point 5 m away
- **THEN** the player runs toward it

#### Scenario: Supersampled view
- **WHEN** the right button is held with the pointer just above the player on screen in the default supersampled view
- **THEN** the player walks up the screen toward that point
