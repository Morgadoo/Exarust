# Design

## Emblem layers

The factory paint job appends, after an area's own layer, a layer with the
same mask, material and wear and the emblem area's colour, and binds the
emblem picture (`emblembkg0NN`) to the pattern unit. The paint job draws a
layer with a pattern through the painted-pattern program (`Mat.TexOpc`:
`alpha *= pattern(vrTex·PtrnPos.xy + PtrnPos.zw).r`), or through the plain
colour program when the material has none, and skips it in the specular
and normal passes. Exarust multiplies the layer's cover by the picture in
the colour pass and skips the layer in the other two.

A picture that does not load drops the emblem layer. This is a guess: the
original's fallback for a missing picture was not traced. Stored choices
replay emblem 0, which has no picture, so stored weapons paint as before.

## PtrnPos

Assembly order, for parts that are not overlays, have a selection and whose
component loaded. The atlas ratio and the factor f = 1 − byte/254 (or
/127 when the byte is negative) are computed in double. The scale and the
re-centring −(f − 1)/2 are computed in float. The offset move
f·byte/127 + offset is computed in double.

## UVScale

The paint job sets 1, then F(−value) for each deform whose whole header
word equals 6. A pass uploads it, and multiplies `ShdTile.y` by it, only
when its program declares it. Only `steelbld_d/s/n` do, so the scale
applies to blade layers only. Exarust paints a region in its own
coordinates, so `ShdTile` reduces to (1, 1, 0, 0) and the stretch becomes
(u, v·UVScale) for the mask, the shade and the region normal. Scale 1 keeps
the previous integer sampling exactly.

## Cost

The work is per texel only for emblem and blade layers, and it runs once
at load, inside the existing cached paint job. Frame time is unchanged.
