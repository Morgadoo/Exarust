# Spec Delta

## MODIFIED Requirements

### Requirement: Shut door
A shut door SHALL NOT make the room behind it active. Every door neighbour of an active room SHALL be adjacent whether its door is shut or open, so the room behind a shut door is drawn under its lid; opening the door SHALL let that room turn active when the anchor is within 2 m of its door point.

#### Scenario: Closed then open
- **WHEN** the anchor stands 1 m from the door point of a shut door, then the door is opened
- **THEN** the room behind the door is adjacent, then active

#### Scenario: Golem gate
- **WHEN** the player stands at the shut double door between exanima06's two golem statues
- **THEN** the room beyond it is black

## ADDED Requirements

### Requirement: Door link state
A door link SHALL count as open, for the active rooms, the reach query and the spill, while it has no door body or its body's pair flag is clear. The pair flag SHALL be set while each leaf of the door is unlocked and shut or less than 0.4 rad from shut (a sliding leaf: at 0.8 of its travel to shut or more), so a locked door counts as open.

#### Scenario: Swing
- **WHEN** an unlocked hinged door is shut, 0.39 rad open, then 0.41 rad open
- **THEN** its link counts shut, shut, then open

#### Scenario: Locked
- **WHEN** a locked door is shut
- **THEN** its link counts as open

### Requirement: Placed door binding
Each placed object of the door kind SHALL be bound at load to its door link with the body of its object node's first child, and each bound link SHALL take its body's state from both of its rooms every frame; a leaf without a body SHALL leave its link open.

#### Scenario: First story level
- **WHEN** the installed exanima01 is loaded
- **THEN** 96 of its 140 door objects bind to a link, each on both sides
