## MODIFIED Requirements

### Requirement: Lifted item out of the simulation
While the item cursor carries an item, its body SHALL be asleep, disabled and at sleep countdown 0, and each placement SHALL copy the placed frame into the held part whole, every other part carried with it; the release SHALL clear the disabled state.

#### Scenario: The release count
- **WHEN** a lifted item is let go in reach
- **THEN** its countdown is 128, and a sleeping body touching its old place takes 128
