## ADDED Requirements

### Requirement: Sampler IK step
The living sampler SHALL fold each leg's stored thigh turn, hip and knee corrections into its clip chain before the torso twist, recording the clip's hip and ankle from the chain before the fold, and SHALL re-place every part under the shin by its clip placement under its re-placed parent. The record SHALL reach the feet routine in world coordinates, put there by the aim after its carry.

#### Scenario: A turned root
- **WHEN** the body root is 3 m along x and turned a quarter about the vertical
- **THEN** the leg's animated hip and ankle are the chain's recorded points under that root

#### Scenario: The foot follows
- **WHEN** a knee correction of 0.2 rad is folded into a shin with a foot 0.4 m under it
- **THEN** the foot keeps its 0.4 m placement under the bent shin

## MODIFIED Requirements

### Requirement: Leg bend
A leg SHALL bend onto its bend target. The knee SHALL be the law of cosines on the two bones: 30° over the double 0.866, 180° at or below −1, else the engine's arc cosine. The correction from the clip SHALL advance at most 0.02 rad a tick (0.2 while stepping) plus its stored rate, never past the target. The rate SHALL become half that sum, within ±0.1 rad. Muscles SHALL follow the correction on the next tick, through the sampler's IK step. Lying down it SHALL release instead.

#### Scenario: Folded to 40 cm
- **WHEN** both bones are 40 cm and the hip-to-target distance is 40 cm
- **THEN** the unsmoothed knee angle is 60°

#### Scenario: Folded too sharp
- **WHEN** both bones are 40 cm and the hip-to-target distance is 20 cm
- **THEN** the unsmoothed knee angle is 30°

#### Scenario: Next tick
- **WHEN** the stored correction and its rate are 0 and the new correction is past 0.02 rad
- **THEN** the pose the muscles follow on that tick is unchanged, and the next tick's correction is 0.02 rad

#### Scenario: Lying down
- **WHEN** the body is down, dormant or dead, not getting up, and a stored knee of 0.2 rad and a hip of −0.2 rad have a zero rate
- **THEN** the next stored knee is 0.18 rad and the hip is −0.18 rad, each rate half of that 0.02 rad step, and the bend target, the step lift and the thigh's extra turn are unchanged

### Requirement: Stance score
The stance score SHALL be 2(1 − C²) + ((1 − W²) + A), the feet's difference taken with its height zeroed. A SHALL be 1 minus the reference's horizontal distance to the foot segment times the single 1/30 (1 under the squared general epsilon), less a double 0.1 a cm of a crouched span past 120 − 30·crouch. W SHALL be |60·scale − span| / (60·scale), unguarded. C SHALL be the same for 55·scale and the span's dot with the frame's side axis, rooted over the epsilon.

#### Scenario: Natural and crossed
- **WHEN** the feet are 60 cm apart, the reference lies on the segment, and the frame's side axis runs from the first foot to the second
- **THEN** the natural order scores 2.524 and the swapped order scores −4.744

#### Scenario: Feet at different heights
- **WHEN** the second foot is 40 cm higher and 60 cm along, and the reference is 20 cm up over the segment's midpoint
- **THEN** the reference is on the segment across the ground and A is 1
