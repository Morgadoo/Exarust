# Proposal

## Why

Backlog item E3 (`context/wiring-backlog.md`): the displacement shell (TShellEffect) and the teleporter scan (TScanEffect) had their timelines, fade planes and pass rules ported, but nothing drew them. Vanish and the Long Door showed no shell and never hid the body; the reconstruction device's scan ticked without a picture.

## What Changes

- `exa_render::force_effects`: the shipped `dispshell_v`/`dispshell_f` ported to WGSL (`disp_shell.wesl`, `DispShellMaterial`): black output with the shell's alpha under the `_EffectShell01` record's blend word 0x163 (premultiplied), one-sided (flags 0x480008), `EmiTex` the record's emissive layer `FlameletsA3`, `TexOfs` rolling over 40 s, `Interior` mixing the outer rim with the darkened inside; a depth-only form for the vanish's segment pass (colour writes off, depth writes on, sorted just before the shell). `effectscan_f` added to `force_shell.wesl` as `SCAN` (fixed 0.03 coordinates, fade term clamped to 100, no ripple points, the `Colour` uniform).
- `exa_game::thaum::vanish_live`: a Vanish or Long Door effect node carries the vanish's phase table and its displacement shell; the shell's state follows the power (closing as the body vanishes, reopening on the come-back, closing with leave to finish at its end), the shell's close calls the table's phase hook, the table's entries decide the segment pass (depth 0x200, fade 0x400) and the body's own draw; the shell is a copy of each of the caster's skinned meshes with its uniforms every frame.
- `exa_game::effects::scan_draw`: a scan node draws the `_ScanAura01` aura over a copy of its body's skinned meshes (`effectscan_f`, `EmiTex` the `_ForceNoise` picture, `TexPos` never set, `Fade` one less the shown fraction, `Colour` times the colour scale); its copies leave with the node.
- The reconstruction device's scan starts with the device's colour scale 3 (it was 1).
- Debug flags `--debug-cast slot@frame[~release]` and `--debug-scan <frame>`.

## Capabilities

### Modified Capabilities

- `thaumaturgy`: the vanish's live shell and body draw.
- `rendering`: the displacement shell and scan programs.
- `level-devices`: the scan's draw, its colour scale and release.

## Impact

- `exa_render::force_effects`, `disp_shell.wesl`, `force_shell.wesl`; `exa_game::thaum::{vanish_live, debug_cast, cast, effect_draw, live}`, `exa_game::effects::{scan_draw, vanish}`, `exa_game::teleporters`, `exarust`.
- Evidence: REA (Ghidra) pseudo-code of the shell's program setup and uniform setter, the scan's, the vanish effect's pre- and post-draw callbacks and segment pass, the vanish power's state writes into the shell, the vanish start, the scanner device's scan start (colour scale 3.0); the `_EffectShell01`, `_ScanAura01` and `_ForceAura01` record blocks read from `Exanima.exe` (blend word 0x163, flags 0x480008, the shell's emissive layer `FlameletsA3.rfi`); the GLSL in `context/re/shaders/shading/`.

## Out of scope

- The fade pass's blended body draw (the body is taken as shown above one half), the per-supersample-layer pass, the depth pass occluding later blended surfaces only (Exarust draws it among them).
- The hidden body's shadow (shadow passes still draw it), held items' own fade.
- A live use of a reconstruction device (nothing sends the use yet); the constant displacement's and the summon's shells.
