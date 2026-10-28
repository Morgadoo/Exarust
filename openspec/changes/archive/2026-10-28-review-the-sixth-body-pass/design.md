# Design

## Where the clip record lives

The original's sampler builds the clip chain, then the IK chain from it
(each part's turn times its leg corrections, placed under its parent's
IK placement), and only then puts the chain on the character frame. The
clip ankle and hip it records come from the clip chain. Exarust's
sampler output is in the body root's frame and the aim multiplies it by
the root after carrying the body, while the feet routine, the standing
and locomotion IK and the walking lag read the record as world points.
The IK step therefore keeps its record in the chain's frame on the leg,
and the aim, after its carry, puts it in the world by the same root it
puts the targets on. A pose given to the aim without a sample (tests,
the inspector) keeps the old path: the record read from the targets and
the bend folded there.

## The chain under the shin

Folding only the thigh and shin left the foot at the clip's placement,
so the foot pitch read a foot turned against the bent knee. Every part
after the shin whose parent was re-placed keeps its clip placement
under that parent, as the original's chain composes it. Parts come
after their parents in the body's list.

## Stance score

The original zeroes the y of the feet's difference before the span, the
segment projection (whose dot it sums x, y, z) and the side dot, and
compares the squared distances with its general epsilon (0x33ffffd7 as
a single). The reference keeps its height; only the horizontal distance
is measured.

## The spawn

The spawn puts the first clip's pose on the parts before the body is
inserted, as the original's placement routine runs its sampler first.
The leg snap that follows in the original is not called: Exarust's
controller keeps its own capsule height rather than the snap's frame
height, and with the snap the standing player settled 4.5 cm lower.
