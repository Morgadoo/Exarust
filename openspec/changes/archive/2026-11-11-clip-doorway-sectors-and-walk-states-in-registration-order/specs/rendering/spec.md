## ADDED Requirements

### Requirement: Registration order within a state
In a level with room sectors the transparent phase SHALL draw the decal and blended surfaces of one state in one sector in the order the sector registered their meshes, and a surface the scene does not hold after them in view-depth order.

#### Scenario: Two cobwebs of one material in one room
- **WHEN** two cobwebs of the same blended material are registered in one room, the nearer one first
- **THEN** the nearer one is drawn first

### Requirement: Portal scissor on the super-sampled view
On a super-sampled scene view each decal or blended surface whose mesh carries the scissor bit SHALL be drawn only inside its sector's doorway rectangle, the rectangle in window pixels doubled to the sub-frame samples and held inside the viewport, and the surfaces after it without the bit SHALL be drawn over the whole view.

#### Scenario: A cobweb in the next room
- **WHEN** a cobweb of an adjacent room is seen through a doorway whose rectangle covers half of it
- **THEN** only the half inside the rectangle is drawn

### Requirement: No portal scissor in a single view
Without super-sampling no surface SHALL be clipped to a doorway rectangle, as the original's queue flush makes no scissor call in a single view.

#### Scenario: Super-sampling off
- **WHEN** super-sampling is off and a cobweb of an adjacent room is seen through a doorway
- **THEN** it is drawn whole
