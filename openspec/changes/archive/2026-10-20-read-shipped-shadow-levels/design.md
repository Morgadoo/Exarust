# Design

`read_mesh` already read the word for versions after 0x3D21; it is renamed
`render_flags`. `Mesh::shadow_bias_level` returns bits 16–18 and
`Mesh::shadow_bias_inflated` bit 19. `ModelLoader` stores the level on
`ModelNode::shadow_bias` (0 for non-mesh nodes and the inflated variant);
`spawn_model` inserts `ShadowBias(level)` on each mesh part when it is not
0. The grid builder spawns its own parts with level 3 or 1, and terrain
tiles keep 3, as the original's builders overwrite the word's level.

The mesh tag already carries the level (bits 0–2) to the receiver side, and
the cube-shadow caster pass already reads `ShadowBias` for k_c, so no
shader changes.

Verified in exanima01's start room with `--debug-column`: the slabs
`uw01_floorbrk03`/`13` lie at −0.013/−0.008 m under terrain at −0.001 to
0.015 m; before, the terrain over them was shadowed; after, it is lit, and
`--no-shadows` matches it apart from the true shadows.
