## MODIFIED Requirements

### Requirement: Hub busy flag
Opening the match board or Manage Company SHALL make the hub busy and closing either SHALL clear it; the trade window SHALL leave the flag alone.

#### Scenario: Board and manage
- **WHEN** the board is opened, then Manage Company is opened and closed with a right release
- **THEN** the hub is busy while each is open and not busy after the close

