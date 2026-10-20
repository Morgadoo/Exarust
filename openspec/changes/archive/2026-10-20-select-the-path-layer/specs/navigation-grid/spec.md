## ADDED Requirements

### Requirement: Path layer choice
The layer choice SHALL take layer 0 when its floor is set and at most four above the current floor, or mark no layer when its floor is five through ten above; a floor more than ten above SHALL be refused. A set layer 1 SHALL win when it is higher and within four. The chosen rise SHALL stay within −20 through +4.

#### Scenario: Layer zero, layer one and refusals
- **WHEN** layer 0's floor is the current floor and layer 1 is unset
- **THEN** layer 0 is chosen at that floor
- **WHEN** layer 0's floor is the current floor and layer 1's is three above
- **THEN** layer 1 is chosen
- **WHEN** layer 0's floor is seven above and layer 1 is unset
- **THEN** no layer is chosen
- **WHEN** the chosen rise is outside −20 through +4
- **THEN** no layer is chosen
