## Why

Roadmap task B4 (lane `forcedraw`, the triage's port lane L06b, force-effect
draw). The ledger rows named a blast strip skipped while it charges, a wave
never drawn (F188), tendrils drawn through a surface-program stand-in with
their scroll unread (F193), an electricity emitter built off screen, a
torrent eye taken as unturned, and a barricade drawn with an additive glow
stand-in instead of its program.

## Evidence

REA (Exanima.exe 0.9.5), the instructions read where constants, order or
branches matter:

- The blast class's frame (shared by the wave's class) builds every live
  chunk's strip in every state; only the colour bytes depend on it (the
  charge and `4c(1 − c)` while charging). The wave's creator makes the
  blast's GI emissive, so the wave lights as the blast does.
- The `_TDispMat01` record's draw method calls the tendril geometry's
  program set-up (`effectforce_v`/`effectsmoke_f`, `EmiTex` on unit 0,
  `TexOfs = (0, scroll, 1)`) and draws through that program's `ObjMat`;
  the scroll is the effect's between its last two ticks, the 1 its
  add-to-world's. The callback's matrix loop is the super-sampling passes.
- The electricity emitter's frame stops after the light and hum when its
  node's drawn bit (0x1000000) is clear: no scroll, ribbon, light points
  or origin write-back.
- The torrent's frame turns the camera's place into its node's rows (a
  rotation, no translation); nothing turns the node.
- The barricade geometry builds `effectfbrcd_v`/`effectfbrcd_f` and binds
  `ViewPos` (the camera in the node frame), `BExt` (the node's half
  extents), `ColPt[8]`, `TexPos` and `Fade` (`barricade_alpha`); the barrier
  record binds `_ForceNoise`; `TexOfs` is the pass offset, 0 with one pass.

## What changes

- `thaum::blast_live::draw_blasts` draws in every state through
  `draw_blast_ribbon`; `thaum::wave_live::{draw_waves, wave_lights}` draw
  and light the wave as a blast (`wave_as_blast`, `WaveLights`).
- `thaum::live::draw_wisps` draws with `ForceRibbonMaterial::smoke` over
  `FlameletsA3`, the second coordinate set and `TexOfs (0, scroll, 1)`;
  `DispTendrilMaterial` and `disp_tendril.wesl` are removed.
- `thaum::arcs::ArcEmitter::frame_seen` and `draw_arcs` gate the ribbon,
  light points and origin write-back on the node's view-frustum test.
- `exa_render::force_effects::BarricadeMaterial` (`force_barricade.wesl`)
  draws the barricade; `thaum::live::barricade_params` binds its uniforms;
  `Barricade::tex_pos` keeps `TexPos`.

## Impact

Pictures change in scenes with a charging blast (its strip shows in the
hand), a wave, a constant displacement or a phased blow (dark smoke
strands instead of bright ones) and a barricade. Lockstep traces change
only when an electricity emitter is off screen (its origins are no longer
put back).
