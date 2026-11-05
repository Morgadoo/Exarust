# Design

The softness source travels from the scene environment to the sun map
through the region resource rather than through the frame, because the fit
already reads `SunShadowRegion::c` in both modes and the region is replaced
with each level. The live environment system writes it after the tick, so
a driven time of day softens the shadow in the same frame as it moves the
body. The value only reaches the caster depth nudge in Exarust: the filter
radius it sets in the original needs a softness input Bevy's directional
shadow filter does not have (as before this change).
