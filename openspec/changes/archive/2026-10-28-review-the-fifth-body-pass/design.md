# Design

The feet routine now measures the lag the carry's push reads: per walking
leg, the animated ankle (the pre-IK clip ankle the bend stores, carried
with the character frame) less the physical contact along the bearing's
facing, which the movement stage already negates while walking backwards.
The push reads last tick's value, as the original's body tick pushes
before its feet routine runs. The swing flag reads the same lead, the
pelvis alignment from the movement decision and the gait's ground
difference from last tick's grounds. Its gate stays Exarust's walking
condition rather than the original's locomoting flag: Exarust's combat
footwork steps set "moving" without the original's step vector, and the
flag changes the feet's drive rates.

The Ground Query is a trait method over the level's pieces with a
`floor_piece` filter. The physics world returns false until the level
colliders carry the floor bit, so the game keeps the miss branch the
earlier review pinned; synthetic triangle lists are floor for the tests.

The placement's leg snap writes anchors, grounds, floors and the snapped
IK corrections and returns the frame height without moving the
controller. It is not called at spawn: run on the freshly built model's
bind pose (Exarust builds the body before its first clip sample) it left
the standing player sinking about 4 cm over 2 000 ticks on the RTX 3090
check, so it waits for a spawn that samples first.

The muscles-off sampler stays a tested function: no Exarust body has its
muscles off without being dead.
