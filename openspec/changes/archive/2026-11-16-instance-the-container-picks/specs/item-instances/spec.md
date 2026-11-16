## ADDED Requirements

### Requirement: Container instantiate
A container child given a row SHALL take, with flags 0, the row's item through the object database's dispatcher, which copies an instance-on-use record into the local tier.

#### Scenario: Two picks
- **WHEN** a chest's two children name tables of an instance-on-use apparel and weapon
- **THEN** the children name two new local copies with flags 0

### Requirement: Container instantiate condition
When the picked record has an item body, the palette SHALL be rolled on the row's wealth, the count byte SHALL take the wealth less its count as a signed fraction, damage and dirt the row's shares of 255 with their spreads, and the apparel walker SHALL run. Otherwise a world or local copy with a weapon section SHALL take damage and dirt and their spreads, and the weapon walker SHALL run.

#### Scenario: The row's condition
- **WHEN** the rows' condition is damage 0.5 with spread 0.25 and dirt 0.75 with spread 0.5
- **THEN** the weapon copy holds damage 128 and dirt 191 with those spreads, and the apparel copy those spreads
