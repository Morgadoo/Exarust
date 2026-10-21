# Design

The original's loader copies a record into a runtime material, rewrites an
alpha-tested blend word to source ONE / destination ZERO (keeping bit
`0x4000`), adds the layer and fall-off bits to the flags, loads the textures
and finally applies the late opaque rewrite on the loaded class and loaded
flags (not masked by the type). The factory then masks the loaded flags by the
type's plug-in (`feature_key`); a word with no low nibble becomes `MatEmi`
when the opacity is above zero, else the record draws nothing.

Two reads therefore differ: the loader-side tests (late rewrite, shadow
caster) use the unmasked loaded flags and the post-alpha-test class
(`loaded_flags`, `loaded_class`), the factory-side tests (premultiply,
emissive, map binding) use the masked key. Only a type-1 plug-in keeps the
`0x300` mode bits, so a type-0 record in raw refraction mode is rewritten
opaque yet still premultiplies.

Compares are in double: `CVTSS2SD` then `COMISD` with a parity branch, so
NaN fails every compare, `0.99f32` and `0.8f32` widen above their bounds and
`0.01f32` widens below. The colour slot (`0x20000`) is bound only for a lit
record; the original shares one slot between diffuse and opacity, which is not
modelled (the opacity map stays separately bound).
