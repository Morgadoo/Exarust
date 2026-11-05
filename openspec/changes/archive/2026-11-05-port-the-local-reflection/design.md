# Design

**No planar reflection.** The task named a planar reflection (mirrored
camera, clip plane, blend). The original has none, so none is added: a
second Bevy view about a plane would cost a full scene pass on a
CPU-bound frame for something the original never draws.

**The low field.** The original's `FrmBuf` for a local reflection is the
post-processor's quarter-size blurred bright pass, left from the previous
frame. Exarust's post chain already produces the same texture
(`exa_post_blur`, sRGB, a quarter of the target), but as a per-view cached
texture in the render world, which a material cannot bind. The plugin
keeps a main-world image of the same size and format, and the post pass
copies its blurred texture into it after the final pass (a quarter-size
copy, about 0.1 MB at 720p). Every local-reflection material binds that
image; the handle never changes, so resizing the window rebuilds the
image in place. The copy comes from the camera with the post chain that
has a target (the window's display camera, or the off-screen frame's).
Feedback: a local-reflection surface's own glow reaches the next frame's
read, as in the original.

**The push.** `uv = n.xy · (1.05/(1.05 − n.z) − 1) · 0.05 · ViewPrj / w +
frag / viewport`, with `w` the clip depth in centimetres (Exarust's world
is in metres, so `w = 100 · depth`), `n` the view-space normal and the y
offset negated for the target's downward texture y. The read is × 1.5.

**The mix.** Inside the GI volume, with a reflection layer and the bit:
`ref = mix(cube · light · Ambient.w · 2, local, a) · gv`, with
`a = min(2·(1 − Gloss), 1)²·(1 − N·V)` and the templates' `Gloss` the
roughness. Outside the GI volume the template has no local read.

**Sub-frames.** The original draws four sub-frames per pass, each at the
window's size with its own jitter ((−0.29, −0.15), (0.30, 0.15),
(0.20, −0.35), (−0.21, 0.35) px), into the quadrants of a double-size
buffer, and averages them. Exarust draws the scene once at twice the
window's size and averages each 2 × 2 block: the same sample count, the
same equal weights, the same fill cost, and the per-sub-frame shadow and
GI taps already split by the pixel's place in its block. The sample
positions differ: an ordered grid at (±0.25, ±0.25) against the
original's grid whose right column sits about 0.2 px lower. No single
raster can produce the original's pattern (its points are not a lattice
that contains the pixel grid), and four Bevy views would multiply the
per-view CPU work (visibility, specialisation, batching) and the sun
cascades by four on a frame that is CPU-bound. The per-sub-frame camera
clock (an animated camera's motion blur) and the shutter depth offsets
are not drawn for the same reason. `EXA_SUBFRAME` shows one sub-frame so
the resolve can be checked.

**The GI cell in the mesh tag.** The original looks the block up from
the node's world bounding-sphere centre. A Bevy mesh's uniform carries
its transform but not its bounds, so the centre is found on the CPU
(the mesh bounds' centre through the transform), only for surfaces whose
transform or bounds changed and for all of them when the block grid
moves; only a changed cell is written. The cell rides in the mesh tag's
top twelve bits (x + 1, z + 1; 0 for none; an off-grid axis 63, which no
block holds). The tag's writer (`lights`) composes it with the shadow
bias and light-list slot, which keep bits 0–19.
