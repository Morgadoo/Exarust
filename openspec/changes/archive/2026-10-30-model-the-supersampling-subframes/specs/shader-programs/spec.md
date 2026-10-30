# Spec Delta

## ADDED Requirements

### Requirement: Super-sampling setting table
The super-sampling setting SHALL index an eight-entry table of sample counts 1, 2, 3, 4, 8, 15, 24 and 66 with jitter offsets 0, 1, 3, 6, 10, 18, 33 and 57. A quarter SHALL draw `min(samples, 4)` sub-frames and a frame `ceil(samples/4)` quarters. Sub-frame `i` of quarter `q` SHALL take jitter entry `offset + 4q + i` with no stop at the setting's own run.

#### Scenario: Three samples
- **WHEN** the 3-sample setting is drawn
- **THEN** one quarter of three sub-frames is drawn

#### Scenario: Past the run
- **WHEN** the 15-sample setting draws its sixteenth sub-frame
- **THEN** it takes jitter entry 33

### Requirement: Sub-frame matrices
Each sub-frame SHALL build a frustum whose edges are the stored negative half extents shifted by `2·half·jitter/size` (`half + d` and `−half + d`), a view matrix that transposes the camera's axes with a translation row of `0 − t` dotted with each axis row summed `(x + y) + z`, and the composite `view · projection` summed `(a·b + a·b) + (a·b + a·b)`. A draw SHALL upload the object's matrix and the object times the composite.

#### Scenario: Rotated camera
- **WHEN** the camera's axis rows are `(0, 1, 0)`, `(−1, 0, 0)`, `(0, 0, 1)` at `(1, 2, 3)`
- **THEN** the view's translation row is `(−2, 1, −3, 1)`

### Requirement: Sub-frame animation clock
A sub-frame's clock SHALL be `i·step + base` with `step = time/samples` and `base = (q·step)·4 + (step·0.5 + time·(−0.5))` in single precision in that order.

#### Scenario: Second quarter at 24 samples
- **WHEN** quarter 1 of the 24-sample setting is drawn over a 0.1 s frame
- **THEN** its first clock's bits are `0xBCFFFFFF`

### Requirement: Per-draw emissive choice
A draw whose material key has the emissive bit and whose geometry asks for a per-draw rescale SHALL take the render object's own emissive scale when the render object's own word has `0x20000`, and the geometry record's scale otherwise, in both draw variants.

#### Scenario: Material key bit alone
- **WHEN** the material key has `0x20000` and the render object's own word does not
- **THEN** the geometry record's scale is used

## MODIFIED Requirements

### Requirement: Motion-blur shutter time
A geometry with the shutter flag `0x20` SHALL, on the super-sampled path, rewrite each sub-frame projection's depth lane to `((n + 0.5)·d / N + lane) − d·0.5` in single precision in that order, with `n = 4q + i`, `N` the setting's sample count, `d` the geometry's shutter length and `lane` the first pass's lane read once before the loop, and SHALL rebuild each composite from the unchanged view.

#### Scenario: Four samples
- **WHEN** four samples cover a 0.1 shutter with lane −1
- **THEN** the first sub-frame's lane is `((0.5·0.1)/4 + (−1)) − 0.05`

### Requirement: Global time lanes
The `Global` block SHALL write `ViewPos.w` and `TimeDlt.x` as the sub-frame offset times the frame scale, and `TimeDlt.y` as `−(1/factor)` times the frame scale, the factor being the setting's sample count. The sub-frame offset SHALL be 0 outside super-sampling and `0.5 − (4q + 0.5)/N` for quarter `q` of `N` samples; the frame scale SHALL be 1 for a frame of at most 1/60 s and `16.666…/ms` otherwise. The exposure SHALL NOT enter these lanes.

#### Scenario: Super-sampled slow frame
- **WHEN** a sub-frame offset of 0.125 is drawn after a 25 ms frame with factor 8 and exposure 2
- **THEN** `TimeDlt` is `(0.125 · s, −0.125 · s)` with `s = 16.666…/25`
