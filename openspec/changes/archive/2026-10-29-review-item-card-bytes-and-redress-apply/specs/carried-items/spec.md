## ADDED Requirements

### Requirement: Re-dress apply order
Applying a re-dress plan SHALL detach the changed hands, then clear the body's weapon part once when its weapon class word has none of the `0x30` bits, then attach the new models, then refresh the body when both hands of the stowed set the plan read are empty, then restart the stance unless the body is in its swap state, clear the attach flag and, for a swap plan, exchange the sets.

#### Scenario: An empty stowed set on a body without weapon bits
- **WHEN** a swap plan whose stowed set is empty changes the main hand of a body in its swap state with class word 0x40
- **THEN** the steps are detach main, clear the weapon part, refresh the body, clear the attach flag and exchange the sets
