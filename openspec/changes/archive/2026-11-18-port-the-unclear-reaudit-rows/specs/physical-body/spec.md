## ADDED Requirements

### Requirement: Twist reset at the get-up
The get-up's start SHALL set the torso twist's followed yaw to 0 and its rotation to the identity before anything else, on its waiting and held returns too.

#### Scenario: A body that fell twisted
- **WHEN** a lying body with a twisted torso starts its get-up
- **THEN** its twist yaw is 0 and its twist the identity
