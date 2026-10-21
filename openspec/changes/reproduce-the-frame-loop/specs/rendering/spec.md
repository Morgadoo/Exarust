## ADDED Requirements

### Requirement: Scene pass order
The renderer's model of the original's scene pass SHALL order a frame as: the clear, the alpha-tested and opaque classes and three hook lists with depth writes, then with depth writes off the decal class and the blended lists under a 2·10⁻⁵ projection depth bias, a hook list, the particles' depth-only pre-pass and their colour, the environment's weather effects and the closing hooks.

#### Scenario: Depth writes
- **WHEN** the pass order is walked
- **THEN** depth writes are on from the alpha-tested class to the third hook list and for the particle pre-pass only

### Requirement: Render list orders
The blended list SHALL be ordered by a selection that takes the lowest 4-bit layer and then the strictly highest opacity, the opacity to beat starting at −1 for each slot and reset neither by a lower layer nor by a first pick by layer; the opaque list SHALL be grouped by program key from its first entry, each slot taking the first unused entry with the current key or else the last unused entry.

#### Scenario: Grouping
- **WHEN** the opaque keys are 5, 1, 2
- **THEN** the order is 5, 2, 1

### Requirement: Super-sampled view size
The view model SHALL draw each super-sampled sub-view at the render-scale size (the window's at the default mode 0; `3·trunc(w/4)`, `2·trunc(w/3)`, `3·trunc(w/5)` or `trunc(w/2)` for modes 1–4) into a buffer twice it, up to four sub-views a pass with the original's jitters, and at the window's size without super-sampling.

#### Scenario: The 4× setting
- **WHEN** a 1600 × 900 view draws at level 3 with the default render scale
- **THEN** four sub-views of 1600 × 900 go into a 3200 × 1800 buffer

### Requirement: View projection
The view model SHALL build the frustum's half height as the near distance over cos/sin of half the field of view and the half width as that times the aspect, and SHALL project a camera point in front of the eye to `size·0.5 + scale·size·0.5·coordinate / −z` with y flipped, a point at or behind the eye to the window edge its coordinates' signs pick.

#### Scenario: 45 degrees
- **WHEN** the field of view is 45° at a near distance of 1
- **THEN** the half height is 0.4142

### Requirement: Framebuffer stacks
The view model SHALL place a viewport from the current region's origin and clip it by the region, refusing one apart from it, and SHALL place a region from the current region's origin less its scroll offset, clipped by it, up to seven regions above the root.

#### Scenario: Region overflow
- **WHEN** an eighth region is pushed
- **THEN** it is refused
