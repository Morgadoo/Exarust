# Design

`CubeCasters` maps a caster to `CasterFaces { faces, centre, radius }`.
`add_cube_shadows`, which already dirtied every face of a moved light, now
also recomputes `faces` for each caster from its stored sphere and the
light's translation. A caster moved, linked or unlinked updates its sphere
and faces as before. The masks are never 0 any more (every caster has a
sphere), so "unknown" no longer exists.

Verified against the build before `cache-character-shadows` with the player
walking at fixed frames: the torch's shadows (the player, crates, walls)
match.
