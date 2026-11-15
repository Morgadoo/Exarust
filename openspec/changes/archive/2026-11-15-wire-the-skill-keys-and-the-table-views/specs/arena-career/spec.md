## ADDED Requirements

### Requirement: Arena tables as table views
The match board's tables and Manage Company's SHALL take presses through their table views: a press on a greyed row or on the selected row SHALL change nothing.

#### Scenario: Greyed tier
- **WHEN** the rank is Inept and the tier list's fourth row is pressed
- **THEN** the tier stays Inept

### Requirement: Arena table drag and drop
The company, lineup and roster tables SHALL begin a drag of the pressed row with the fighters' drag kind; a release SHALL offer the drop to the table under the pointer, which takes its own kind when its accept check passes and runs its drop callback.

#### Scenario: Drag onto the lineup
- **WHEN** a company row is dragged onto the lineup and the rank check passes
- **THEN** the fighter joins the lineup

### Requirement: Arena table selections drawn
The board's and Manage Company's tables SHALL draw their selected rows' bands, and a drag moved past the drag distance SHALL draw its label at the pointer; Manage Company's roster SHALL open with its first row selected.

#### Scenario: Board opened
- **WHEN** the board opens on the standard tab
- **THEN** the match, rank and tier lists draw a band and the company table none until a row is pressed
