## ADDED Requirements

### Requirement: The idle look chain on the locomotion
A brain on its locomotion without the in-combat mark SHALL run the out-of-combat look chain every tick it stands, also while the staging freeze holds, and its body's heading SHALL change only when the chain's look lies more than 30° from it; its model SHALL NOT be turned by the AI.

#### Scenario: A frozen brain
- **WHEN** a brain on its locomotion facing 0 and looking along 1.0 ticks once under the staging freeze
- **THEN** its model is not turned, its look eases toward 1.0 and its request's heading stays along 0

## MODIFIED Requirements

### Requirement: The idle turn
While not engaged, a brain's eased yaw SHALL start at the body's facing and ease toward the look plus a sway by `0.09 − 0.06·encumbrance`. The sway is `u·3.6 − 1.8`, redrawn every 4000 ms plus up to 4000 ms. A body without muscles SHALL have its model turned to the eased yaw; a body on its locomotion SHALL turn by the heading its look chain writes.

#### Scenario: Easing toward the look
- **WHEN** an idle brain without muscles faces yaw 0 and looks along π/2 with no sway and no encumbrance
- **THEN** its model turns to 0.09·π/2 that frame
