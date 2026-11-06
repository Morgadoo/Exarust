## ADDED Requirements

### Requirement: The out-of-scene slide
While a roaming character is out of the scene and follows a route, its roam SHALL keep running and its standing point SHALL slide 2 cm a tick along the route with no physics; the leg SHALL end within 100 cm of the route's last point.

#### Scenario: A walker leaves the range
- **WHEN** a walker leaves the activation range mid-route
- **THEN** its standing point moves 2 cm a tick along the route until the leg ends

### Requirement: The re-entry placement
When the scene takes a character back, its clip SHALL snap to its stance and its balance radius base SHALL be 60 cm with its counter 0, its standing point SHALL move to the nearest walkable point, and its body SHALL be placed on the ground there facing its body's yaw, or its route's goal while it follows one, its frame taken afresh and its yaw, heading and look set to the placed yaw. A standing character's turn SHALL go through its heading at the body's turn rates.

#### Scenario: A slid walker comes back
- **WHEN** a slid walker re-enters the range
- **THEN** its body stands at the slid point and does not move back to where it left

### Requirement: The driver stops for a body in the way
After the driver writes its request, a live nearby character that the walk reaches within its 120 cm steer line (its point moved 20 cm along the walk) SHALL stop the walk and end the route, once until the next planning turn.

#### Scenario: A character ahead
- **WHEN** a character stands 1 m ahead on the walk
- **THEN** the walk soft-stops and the route ends; the next check skips it
