## ADDED Requirements

### Requirement: Body material inspection
`exa-tool bodies` SHALL print, for each rigid body, the material of its node's first mesh part, the divisor its mobility is divided by when it registers, the mobility it registers with and its impact sound set.

#### Scenario: An iron-banded barrel
- **WHEN** `exa-tool bodies Objlib "barrel ubm a01"` runs
- **THEN** its body shows material `mtliron04`, divisor 3, registered mobility 0.00353 and impact sounds `mtlcol`
