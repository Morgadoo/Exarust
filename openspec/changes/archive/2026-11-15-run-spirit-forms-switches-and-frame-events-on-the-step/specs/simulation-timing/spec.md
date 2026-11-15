## ADDED Requirements

### Requirement: Free step events run after the frame's last packet
With the free mode on, the frame SHALL run the events of every step packet it applied in a frame after the last of them, in send order, so that no later apply settles an event's writes, spawns, despawns or messages as its own. Every bridged change an event makes SHALL reach the simulation in the frame's next packet.

#### Scenario: Several steps in one frame
- **WHEN** each of a free simulation's steps sends an event that spawns a shared entity and writes a shared message, and frames apply two or three steps each
- **THEN** the simulation holds every spawned entity and hears every message

## MODIFIED Requirements

### Requirement: Events to the frame
Events the simulation sends during a step SHALL run on the frame's world after that step, in the order they were sent, each once. With the free mode on they SHALL run after the last step packet the frame applies in that frame.

#### Scenario: Two steps
- **WHEN** a step sends events A and B
- **THEN** the frame runs A then B after that step, and neither after the next
