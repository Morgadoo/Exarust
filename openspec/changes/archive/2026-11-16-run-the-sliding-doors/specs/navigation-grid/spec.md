## ADDED Requirements

### Requirement: Sliding leaves as movers
A sliding door's leaf SHALL stay a mover of the navigation grid when it registers as a body; a refresh over the rectangles of where a shut leaf was and where it now stands SHALL free the cells it left and raise the floored cells under it.

#### Scenario: Portcullises shut
- **WHEN** `exanima01`'s unlatched fast portcullises are shut by signal and their two rectangles each are refreshed
- **THEN** cells they left are lowered and doorway cells under them are raised
