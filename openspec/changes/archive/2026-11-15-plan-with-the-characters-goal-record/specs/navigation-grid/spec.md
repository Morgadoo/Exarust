## ADDED Requirements

### Requirement: Planner goal record
A search SHALL take its size class, penalty shift and accept distance from the caller's goal record. The AI's record SHALL start at size class 3 and accept 10, and its shift SHALL be the group-alert count.

#### Scenario: Size class 3 keeps off low clearance
- **WHEN** a corridor has a cell of clearance 50 and a search crosses it with size class 0, then with size class 3
- **THEN** class 0 reaches the goal and class 3 stops before that cell

### Requirement: Planner end test
A search that stops short SHALL ask the record's end test whether its closest node's centre, in centimetres, is too far from the goal; only a record without one SHALL use the 100 cm rule. An end the test accepts SHALL keep the whole path to the closest node.

#### Scenario: An end test that accepts
- **WHEN** a search stops one cell short with a rough cell before it and the end test accepts every end, then refuses every end
- **THEN** the first plan is reached and ends at the closest node, the second ends before the rough cell

### Requirement: The states write the goal record
Before it plans, each AI state SHALL write its fields of the record: the roam and patrol size class 4 and the pick's accept 20 with the flat test at 150 cm; the chase the phase word plus 3, accept 10 and the half-height test at 150 cm, 300 cm for a downed target; the flight the phase word plus 3, accept 30 and the flat test.

#### Scenario: A chase by a marked body
- **WHEN** a body whose phase word is -1 re-plans its chase of a standing foe
- **THEN** its record holds size class 2, accept 10 and the half-height test at 150 cm

### Requirement: The follower, search and Long Door records
The follower's walk to the leader SHALL write size 0, accept 50 and the half-height test; its closing walk size 4, and a goal moved off the leader accept 20 with a test that never fails. The search SHALL write size 7 and accept 50. The Long Door's query SHALL write accept 50, size 3 and the never-failing test, and set the alert count to 1.

#### Scenario: The Long Door's query
- **WHEN** an AI caster's Long Door asks the grid for a place behind it
- **THEN** its brain's record holds size 3, accept 50 and the never-failing test, and its alert count is 1

### Requirement: Line walk end projection
A line walk or straight walk blocked after its first step SHALL end at the cell point before the block, its minor axis unrounded, projected in centimetres onto the start-to-end direction normalised with the processor's reciprocal square root estimate, keeping the asked height.

#### Scenario: A wall across a slanted walk
- **WHEN** a walk from cell (1, 3) to cell (12, 6) meets a wall at x = 8
- **THEN** the line walk and the straight walk end at the projection of the point (7, 30/11 + 21/11), within a millimetre of it
