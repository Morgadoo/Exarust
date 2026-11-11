## ADDED Requirements

### Requirement: Decal and blended list depth bias
A surface with a decal or blended render state SHALL draw with the original scene pass's projection bias: the first view's projection `[2][2]` term plus 2·10⁻⁵ added in double and stored in single, expressed as the matching change of inverse view distance for the original camera's planes (near 100, far 12 500 engine units) and added to Bevy's reverse depth scaled by the view's near plane on perspective views.

#### Scenario: The stored step
- **WHEN** the original camera's term −12 600/12 400 is biased
- **THEN** the stored term moves by 2.0027·10⁻⁵ and the depth shift is 9.9335·10⁻⁶ per metre

### Requirement: Unbiased surfaces
Surfaces without a decal or blended render state SHALL draw with Bevy's mesh vertex stage unchanged, and `--no-list-depth-bias` SHALL turn the bias off for every surface.

#### Scenario: An opaque surface
- **WHEN** an opaque surface's pipeline is specialized
- **THEN** its vertex stage has no list depth bias
