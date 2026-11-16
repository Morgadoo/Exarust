## ADDED Requirements

### Requirement: Cloth body contacts in the solver
The cloth's body contacts SHALL run inside the bodies' solver after body–body detection and before body–level detection, the friction passes and the post-step, one cloth at a time in the object list's order, so a later cloth and a later sphere meet a body where an earlier push left it. A cloth's own step SHALL come before the solver and its pass after the collisions and its level contact after it.

#### Scenario: A sleeping body under an awake sheet
- **WHEN** a sleeping one-part body's sphere reaches 5 cm up through an awake pinned sheet
- **THEN** in that tick the body is pushed down by 0.002 of the sheet's summed push and woken, and with the contacts off it stays asleep where it was

### Requirement: Cloth body share
After each body sphere that pushed the cloth, the body SHALL take the sphere's summed push times 0.015 (an active striker part), 0.01 (a cloth on a held chain) or 0.002, in metres, on its particles by the sphere centre's raw weights in the part's rest tetrahedron.

#### Scenario: A still part over a triangle
- **WHEN** a still part's 3 cm sphere stands 3 cm over a cloth triangle
- **THEN** each particle moves by its weight times 0.002 of the summed push, and the cloth's pushes are those of the fixed contact

### Requirement: Cloth body slide
A pushing sphere SHALL rub the body's motion record at its centre: a squared speed over the small float SHALL add `v²·2e-5` to the cloth's rustle tail (double), the speed SHALL lose its part along the push, and SHALL be negated when `0.2·push² ≥ v²` (double, or unordered) or else scaled by `−√(0.1·push²/v²)`.

#### Scenario: Across the push
- **WHEN** a body moves (3, 0, 4) cm at a sphere whose share is (0, 0, −1) cm
- **THEN** its motion there takes (3·k, 0, 0) with k = −√(0.1/25), and the tail takes 25·2e-5
