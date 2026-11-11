## MODIFIED Requirements

### Requirement: Cell meeting order
The body–body detection SHALL meet, for each awake part, the parts of its nine cells (its own first) in the order a cell's list holds them: the part linked into the cell last first. A body SHALL be linked into its cells when it joins the list, a node before its children.

#### Scenario: Two parts in one cell
- **WHEN** an awake part's cell holds two parts that never left it
- **THEN** it meets the one that joined last first

## ADDED Requirements

### Requirement: Cell migration
Before each detection, every awake part whose cell changed SHALL be linked at the head of its new cell's list, in the awake list's order; a sleeping part SHALL keep its cell entry until it is awake again.

#### Scenario: A part comes back
- **WHEN** a part that joined first leaves its cell and comes back while a later part stays there
- **THEN** it is met first again

#### Scenario: A sleeper is moved
- **WHEN** a sleeping part is moved into another cell
- **THEN** it keeps its entry, and takes the head of its new cell on its first awake tick

### Requirement: Velocity in the third set
Giving a body a velocity SHALL place both its previous particles and its third particle set a tick's travel behind its current particles, so the two-step point reads the velocity.

#### Scenario: A body given a velocity
- **WHEN** a body at rest is given 0.83 m/s along x
- **THEN** the two-step point of any point is 1 cm along x
