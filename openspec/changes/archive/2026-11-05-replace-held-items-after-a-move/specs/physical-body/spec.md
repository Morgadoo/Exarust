## MODIFIED Requirements

### Requirement: Sole turn
In each iteration, after the sole push, a body on its feet with support SHALL turn a foot with a contact state whose forward lies within a right angle of its animated forward. The horizontal gap from the toe to where that orientation places it, projected onto the leg's sole side axis, is closed by the velocity servo at rate 0.4×scale and gain P×pin factor×S. A gap under 1.19e-7 cm², one already closing 0.05 of itself, or a foot with no contact state, is left. P is the support pins' power.

#### Scenario: Two centimetres to the side
- **WHEN** a standing foot's toe is 2 cm along its animated side axis from where that orientation places it, it has not moved, its forward agrees, S is 1, P is 1 and the pin factor is 1.25
- **THEN** the toe is pushed 1 cm along that axis

#### Scenario: No contact state
- **WHEN** a foot with no contact state has its toe 2 cm along its animated side axis, it has not moved, and its forward agrees
- **THEN** the toe is not pushed

## ADDED Requirements

### Requirement: Sole side axis
The sole turn's side axis SHALL be the foot's animated side axis, taken and kept on the leg each iteration, except while the body moves and the foot contact touches without being lifted, when the kept axis SHALL be used and left as it is.

#### Scenario: Planted while moving
- **WHEN** a moving body's foot touches without being lifted and its leg kept the +z axis
- **THEN** the push lies along +z and the kept axis is unchanged; with the foot lifted, the orientation's own side axis is taken and kept
