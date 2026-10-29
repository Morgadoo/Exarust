## Context

The CPU side already gives every value the programs need: `Blast::frame` and `Torrent::frame` the `TexOfs`, `RibbonMesh` both coordinate sets and the colour bytes, `shell_uniforms` the shell's `LclMat`, `ViewPos`, `FdPln`, `ColPt`, `TexPos`, `ColourInt`, `TexScale` and `Fade`, and `visuals::noise_field` the noise object.

## Decisions

- **Bevy materials, not raw passes.** Both programs are `Material`s on Bevy's mesh pipeline, like the arcs and the smoke. The ribbon keeps Bevy's mesh vertex stage (it is `effectforce_v` up to the colour scale); the shell has its own vertex stage on Bevy's skinning so the copy can move out along its skinned normal (2 cm, the skinned copy's `position + 2·normal`).
- **Blend from the record.** The records' blend word 0x163 maps through the original's factor table to (ONE, ONE_MINUS_SRC_ALPHA), Bevy's premultiplied mode; blended draws write no depth. `effectforce_f` never writes its alpha; it is taken as 0 (purely additive). Outputs are clamped to [0, 1] as the original's 8-bit target clamps them.
- **Local frame.** Exarust's world is in metres. The shell's uniform carries `LclMat · scale(100)`, so the fragment stage works in the original's centimetres; the node's frame is unturned at the body's root (the shell node is created unturned) and the skin box is the body parts' spheres' height span.
- **Skinned copies.** The aura copies each skinned mesh below its caster (body and apparel), sharing the source's `SkinnedMesh`, as children of the source so they sort and hide with it; one material an aura holds the uniforms. A copy whose aura is gone is despawned.
- **Flat frame time.** The noise system runs only while a shell aura, a blast-family effect or a torrent exists; the draw systems iterate empty queries otherwise.
- **`EmiTex` for the ribbons.** The force record names no texture the routines make; the only picture the Force unit makes is `_ForceNoise` (mipmapped, which `effectforce_f`'s `textureLod` needs), so it is bound (a guess, recorded).

## Risks

- The per-fragment viewer direction (the original normalises it per vertex) and a per-frame draw instead of one per supersample layer can differ slightly in brightness.
- Body meshes with compressed or missing normals would fail the shell pipeline's specialisation; Exarust's body meshes carry plain normals.
