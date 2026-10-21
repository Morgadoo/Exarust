## MODIFIED Requirements

### Requirement: Panel entries
A panel entry SHALL not be drawn while dragged, held or without an icon; it SHALL be tinted green by the green mask, yellow at half opacity by the yellow mask alone, green at half opacity by both, else black. A looted body's worn panel SHALL leave out every worn item or held shield the icon cache cannot draw, without drawing random numbers for it, scatter the rest over the middle half of the panel and push them apart over four rounds, keeping every centre inside.

#### Scenario: The tints
- **WHEN** entries flagged with the green mask, both masks, the yellow mask and neither are drawn, and a held one
- **THEN** they are green, half-opaque green, half-opaque yellow and black, and the held one is not drawn

#### Scenario: An item without an icon
- **WHEN** a looted body wears two items and the cache cannot draw the first
- **THEN** the panel holds only the second, placed where it would be had the body worn it alone
