## ADDED Requirements

### Requirement: Parallax
The system SHALL displace the texture coordinates of materials that have a height map and a normal map, as the original does. The depth is a fixed number of height-map texels set by the height layer's amount. The shift follows the tangent-space view, in four accumulating steps.

#### Scenario: Floor relief
- **WHEN** the menu floor is seen at an angle
- **THEN** its bricks shift against their grooves with the view instead of lying flat

### Requirement: Displacement shadows
The system SHALL shadow lights by the height map on materials whose extra flags ask for it. Taps are marched toward each light in the tangent plane, and the light is lit by the share of taps the surface stands above.

#### Scenario: Grooves in torchlight
- **WHEN** a torch lights the floor at a low angle
- **THEN** the grooves between the bricks fall into shadow on the side away from the torch
