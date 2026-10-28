## ADDED Requirements

### Requirement: Live static collider grid
Every fixed level collider the static builder creates SHALL be recorded in the sector's 600 cm static grid with its world box in centimetres, its piece and its level mask: 1, plus 4 when its local half height is under 1 cm and its world up component is over 0.99. Props' stand-ins and pick-only geometry SHALL NOT be recorded. Body detection SHALL skip a piece whose mask shares a bit with the part's body mask, and a piece with no record SHALL answer 1.

#### Scenario: A flat horizontal mesh is a thin plate
- **WHEN** a flat 2 m plane, a 1 m cube and the plane turned a quarter about x are built as static colliders
- **THEN** their masks are 1|4, 1 and 1

### Requirement: Living foot mask
A part SHALL carry the foot mask 4 in level detection only while it is a foot of a body with muscles; a limp body's feet SHALL carry no foot mask.

#### Scenario: A limp foot rests on a thin plate
- **WHEN** a limp one-part body marked as a foot falls 1 m onto a floor of mask 4
- **THEN** it rests above the floor, and given muscles its mask is 4, which shares a bit with the floor's

### Requirement: Simulated object stages
A simulated object SHALL take a body part when the two bound spheres are not apart (the sign of `(r + r')² − d²`) and its centre's squared distance outside the part's mesh box is not at least its radius squared, so a NaN passes both tests. Its static stage SHALL take the static records of the nine cells around its own cell under the same two tests against each record's box.

#### Scenario: The mesh box decides after the spheres
- **WHEN** an object of radius 20 cm meets a body whose bound sphere is 15 cm away, with its mesh box 15 cm and then 20 cm away
- **THEN** the first is taken (225 < 400) and the second is not (400 <= 400)

### Requirement: Simulated object pairs
The object/object stage SHALL offer each object taking it every other offered object of the list, in list order, never itself, with no bound test in the sector.

#### Scenario: Every ordered couple
- **WHEN** three objects take the stage as (yes, no, yes) and are offered as (yes, yes, no)
- **THEN** the pairs are (0, 1), (2, 0) and (2, 1)
