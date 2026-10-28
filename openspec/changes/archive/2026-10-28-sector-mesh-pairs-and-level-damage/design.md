# Design

## Mesh query and fallback

The pair stage mirrors the level detection: each of A's spheres is put in
B's frame by the previous and current relative transforms, swept when it
moved over 0.1 cm², then pre-tested against B's mesh box and mesh sphere
before B's triangles are tested one at a time through the shared
`contact::sphere_hit`/`swept_hit`. The original's pair path, unlike the
level path, never reads the static-only bit. No loader hands part triangles
over yet, so the game runs the sphere fallback; it pre-tests against the
bound of B's spheres (`Pairing::sphere`), which can never reject a sphere
the fallback would hit, where a node box might.

## Level impacts

The body's level detection already made `LevelImpact` records. A system
after `strike_limbs` feeds each to the same limb rules: the counter, the
root's clock, `limb_impact_sound`, `limb_impact_damage` with the overhead
drop, the health take and daze shove, and the death/collapse path. The
original's call passes no other body, so the step that grows the other
character's combat concern (Exarust's `LimbImpact`) is skipped.

## Risk

Level damage changes the running game: a hard fall now hurts. The damage
gate needs a volume over 0.25, a scaled strength over 5 (about 30 cm² a
tick² for a standing character, a vertical landing near 5.5 cm a tick).
