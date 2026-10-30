# Design

## The switch

The original compiles `Env.Outdoor` into its programs; Exarust keeps one
pipeline per material and passes the switch as a material flag
(`OUTDOOR`, bit 30 for the world material, 64 for the terrain), set by the
same systems that point materials at the GI volume, from a `GiOutdoor`
resource the level load inserts. A change of the resource revisits every
material, so a cached material reused by the next level follows it.

## The cube

`GiOutdoor` also carries the level's cube (`genref01r` or `genref02r`), and
a separate system binds it to every material that has a cube, whether or
not GI is on, as the original's environment binds one cube for the scene.
Materials are cached across levels, so the indoor cube is bound back when
an indoor level loads.

## Order of operations

The shader keeps the original's grouping: the light times
(`Ambient.xyz`·b·occ), then times (cube·(`Ambient.w`·5)). `Ambient.xyz`
is the level ambient passed ×16 to Bevy, divided by 16 and times 15.
`Ambient.w` is a shader constant per branch (8, 3), the two values the
environment holds.

## Not followed

The mask tint of the outdoor cube is a guess about timing and record; the
bound cube is untinted. On the brightest face the tint would lower red and
green by up to about a quarter.
