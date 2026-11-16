## ADDED Requirements

### Requirement: Plain mesh painting
A wound SHALL paint a plain (rigid) mesh in the mesh's own frame, its centre and travel taken into that frame and its radius kept, so a scaled node takes the wound smaller. The paint SHALL run once per position element, one random draw for each element inside the wound, and every vertex of the element SHALL take its value.

#### Scenario: A seam
- **WHEN** a wound reaches two vertices a seam splits at one place
- **THEN** both take one value, from one random draw

#### Scenario: A scaled node
- **WHEN** the same wound is painted on a node scaled by 2 about the contact
- **THEN** it reaches the mesh's vertices as far in the mesh's frame as on the unscaled node

### Requirement: Cloth painting
A wound SHALL paint a cloth under the head (long hair) in place on the mesh its simulation moves, by the cloth's painter: the point moved back by half the travel, the travel normalised, the point alone taken into the node's frame, every drawn vertex painted as the wound painting does.

#### Scenario: Hair cloth
- **WHEN** a head wound reaches a hair cloth hanging from a turned head node
- **THEN** the cloth's own mesh keeps its handle and takes the blood where the node's frame puts the wound
