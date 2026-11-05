# Design

## The spawn

`limp::spawn_pose` samples the first clip with the pose-only key pick, as
before, and asks the body for the pose-only sampler's pelvis target
(`Ragdoll::placement_target`: the stand height of the posed foot contacts
less the sole, plus twice the scale plus 5 cm; the Unconscious 1 clip
leaves the stand out). The pose is lowered by the pelvis's posed height
onto the model root raised by that target less 8 cm. A human's first Idle
sample stands its pelvis about 1 cm under the clip root's height (101.0
against 101.9 cm, target 109.0, on exanima01), where Exarust used to put
it at the clip root's height.

## The ground placement

The player's behaviour (`player::steer_player`) asks for it on its third
tick: `Steering::snap` answers the grid's nearest walkable point to the
frame's origin (the capsule's place across the ground at the pelvis's
height), and the body gets `limp::GroundPlacement { floor }`, the point's
height. `limp::place_on_ground` runs next in the fixed pre-update: the
current clip sampled by the pose-only key pick, then
`Ragdoll::place_on_ground` with the model root's frame put at the floor.
That runs the leg snap from the floor frame (its IK reads the character
frame the body stands on, as the original's does before it copies the
new frame in), puts the parts at the pose lowered onto the frame at the
snap's height, re-anchors the feet (`placement::ground_foot_anchor`) and
restarts the slack links.

The frame-height write-back: the original's frame sits at the pelvis and
follows it every tick. Exarust's controller is the capsule at the feet,
which its physics keeps on the floor; the snap's height reaches the body
through its parts alone, and the capsule keeps its place. Moving the
capsule to the snap's ground (the seventh body pass's try) made the
squat; it is not done.

Exarust's own: the count waits for the body (Exarust builds it a few
frames after the character, so the third behaviour tick could come
before it); a placement whose snap finds no ground leaves the body alone
(the original then places the pelvis at the floor frame's own height,
which Exarust's partial Height Query could reach on ground it does not
cover).

## Not done

The ground placement's pins (the chest as the mean of the pins on the
frame), its stored feet centre, cloth and attached bodies, and its
behaviour hook; the arena loadout's restart of the count and the mind
swap's skip; the other callers of the ground placement (teleports,
summons, saves, the AI's push out of a blocked region, the end of a
session).
