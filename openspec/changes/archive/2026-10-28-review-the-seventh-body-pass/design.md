# Design

## The placement height

The original's placement matrix is its pelvis-high character frame;
Exarust's model root is at the feet. The mapped spawn would move the
controller and the model root to the snap's averaged character ground
(`Ragdoll::placement_feet`) rather than to the frame height the snap
returns, and sample the snapped frame again. This is a unit mapping,
labelled as such in the code. Live, it left the player squatting after
spawn, so the spawn does not call it; it stays a tested rule.

## The trailing-foot request

The original's start writes the request at once, before the body tick's
movement decision, whose move branch zeroes it. Exarust's animator
records the replaced Walk or Run and the sampler stage, after the
movement stage, takes it; it leaves no request when this tick's decision
moved the body, which matches starts made by the behaviour and the
attack stage. The carry stage plants after the sample (request 2 → 1);
the sampler drops a request strictly past its deadline and raises a
one-tick notice for the behaviour.

## The thigh posture

The in-combat branch calls the posture with the combat flag only on a
standing, non-walking condition; walking or off its feet it calls it
without, as does the out-of-combat branch, which then splays or keeps
the clip's turns by the state word.
