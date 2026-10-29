## ADDED Requirements

### Requirement: Link-mobility blend of an articulated prop
A rigid body's link mobility SHALL start at its registered mobility. When an articulated prop registers, every body with children SHALL run a mask pass over its subtree after its children's own passes. In a pass each child, after its own subtree, SHALL take `parent·0.5 + child·0.5` with the parent's value from before the parent is blended.

#### Scenario: A three-level chain
- **WHEN** a prop of mobility 1 with a child of 0.5, a grandchild of 0.25 under it and a second child of 0.125 registers
- **THEN** the link mobilities are 1, 0.75, 0.4375 and 0.5625

### Requirement: Readers of the blended link mobility
The link shares SHALL keep the values taken before the mask passes. A soft weld SHALL split by the two link mobilities as they stand when it runs. A character's limbs SHALL keep 0.175 of their inverse mass, unblended.

#### Scenario: The chain's child link
- **WHEN** the three-level chain has registered
- **THEN** the child's link shares stay 0.5/1.5 and 1/1.5

### Requirement: Link-share recompute
A recompute SHALL give every link that is a body link and not disabled the shares m/(m + m') and m'/(m + m') of its part's and parent's link mobilities as they stand; other links SHALL keep theirs.

#### Scenario: Recomputing the blended chain
- **WHEN** the three-level chain's shares are recomputed
- **THEN** the child's link takes 0.75/1.75 and 1/1.75 and the grandchild's 0.4375/1.1875 and 0.75/1.1875

### Requirement: Breakable grab gate
A breakable grab SHALL be worked in centimetres from the gap d between the struck point and the weapon point, doing nothing unless |d|² is at least the general epsilon. A disabled struck body SHALL then break it. A re-anchoring grab SHALL instead move its struck point onto the weapon point once and drop the flag.

#### Scenario: Points together
- **WHEN** a re-anchoring grab's points coincide
- **THEN** nothing changes and the flag stays

### Requirement: Breakable grab strain and pull
With q = (d in the weapon's frame)·axis + 1, the strain SHALL fall by (q/|d|)·2 + 1 when q is above 0 and by 1 otherwise. An ordered strain below 0 SHALL break the grab. Otherwise, when |d|² is above the epsilon, the weapon and the struck part SHALL move by their shares m/(m + m') and −m'/(m + m') of their link mobilities times strength/(|d|·0.001 + 1) worked in double.

#### Scenario: A grab along its axis
- **WHEN** a grab of strain 5 and strength 1 between a weapon of link mobility 1 and a part of 3, 10 cm apart along its axis, runs one pass
- **THEN** the strain is 1.8, the weapon point moves a quarter and the struck point three quarters of 1/1.01 of the gap toward each other

### Requirement: Dropping a broken grab
After a break, every breakable grab to that part SHALL be dropped in order, and the grab layer bit SHALL be lowered on that part, its parent, its children and the weapon.

#### Scenario: A spent grab
- **WHEN** a grab of strain 1 breaks while another breakable grab and a plain link hold the same part
- **THEN** both breakable grabs are dropped, the plain link stays and the grab layer bits are cleared

### Requirement: Node-clock render fraction
The node-clock fraction SHALL be 1 − ((now − stamp) − offset·scale)/period, held at −2 from below, when the clock has reached its stamp; ((stamp − now) + offset·scale)/period + 1 when the stamp is at most a period ahead; and offset·scale/period + 2 further ahead. A body's render fraction SHALL be its clock's tick count less its own tick stamp plus that fraction, or 0 while it rests.

#### Scenario: A clock past its stamp
- **WHEN** the clock is 5 ms past its stamp with an offset of 1 ms over a 12 ms tick
- **THEN** the fraction is 1 − 4/12
