# Design

The two leg IK routines share one per-tick record (`IkTick`): the
standing smoother's rate (0.02, or 0.2 while the recovery step runs), the
locomotion rate (0.04 × the gait speed + 0.01, in double), the lift term,
the run flag, the stance, the crouch and the character frame. The feet
routine calls the locomotion IK while the body locomotes and the
standing IK otherwise; its snap argument is always clear, so both
routines ease their corrections. The standing IK keeps the earlier,
reviewed targets (the anchor, or the contact/anchor midpoint at the
anchor's height) and adds the double 30° fold compare, the engine's arc
cosine and the foot's height over its floor for the pitch.

The locomotion IK works in centimetres as the original does: the hip's
height over the anchor plus the per-leg lag (0: the standing IK clears it
and the locomoting value is not reproduced) against the clip's hip over
its ankle; the target leg runs from the physical hip across to the
contact. The hip angle's sign compares the clip's foot in the character
frame (its local z above 0 keeps the angle). A leg the IK does not act on,
or whose target has no length, shows the identity correction while its
stored angle eases back.

The jolt runs in the balance before the recovery step, from the chest's
two smoothed velocities, in metres a tick converted to centimetres for
the thresholds.
