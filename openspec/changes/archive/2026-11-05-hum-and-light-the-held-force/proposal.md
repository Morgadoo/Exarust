# Proposal

## Why

Backlog item E6 (`context/wiring-backlog.md`): the kinesis hold, its shell and its pick sound and the Force Strikes aura were ported, but the kinesis shell's hum, the held body's GI box light (TCuboidEmissive), the strikes aura's GI line light (TStrikesEmissive) and the ripple a strikes caster's landed blow puts on its weapon's aura were not.

## What Changes

- `exa_combat`: a new `StrikeHooked` message, the striker's own hook on a landed blow (the original's damage routine calls it with the striker's primary weapon, the contact and a strength): after a clash's or a block's notice with the impact at the hit, and at the end of a blow on flesh with the yellow damage before its 0.75. A turned blow on flesh does not reach it.
- `exa_game::thaum::effect_draw::ripple_strikes`: while Force Strikes runs, the hook ripples the aura on the caster's primary weapon at the contact (in the weapon's frame) at half the hook's strength, with the aura's `thmfrc_shi` at its gain 1.5 and base pitch 0.8 when its 400 ms gate is open.
- `exa_game::thaum::shell_hum`: the kinesis pick names `thmfrc_knsdr` on its shell (`ShellHum`); joining the scene the shell takes a looping voice at volume 0 and pitch 1, its start `r·0.8` of the buffer's length in (`r` the effect stream's draw); each frame the voice follows the shell node's box centre and the shell's volume and pitch; it is freed with the shell. A throw handed the kinesis's shell keeps the hum.
- `exa_game::thaum::effect_light::shell_lights`: every held shell (the kinesis pick's and the throw's grab) carries its `gi_cuboid` box light (`HeldBoxLight`): the shell's light colour (no manager scale), extent `max(half − 20, 10)`, `bMat` the inverse of the node's frame less the box centre, the node's world box plus 80 cm. Every strikes aura carries its `gi_gradline` line: the weapon node's y segment from `max(c − w, bottom)` to `min(c + w, top)`, its gradient `(c/l, ((1 − c) + y₀)/w/g, ((1 − y₁) + c)/w/(1 − g))`, attenuation 10000, the shell's light colour times one less the node's fade, a point light at its first ripple ten times its shown amount, the node's world box plus 60 cm. Both leave with the shell's light.
- `exa_game::thaum::strikes_span` / `strikes_line`: the strikes centre and start in double where the original takes them (the 0.7 compare and raise, the box's 0.6 and 0.15); `strikes_gradient` uses it.
- `effects::emissive::cuboid_light`: the extent is the engine's MAXSS (10 for a NaN).

## Capabilities

### Modified Capabilities

- `thaumaturgy`: the kinesis shell's hum, the held shells' box light, the strikes aura's line light and ripple.
- `combat`: the striker's hook on a landed blow.

## Impact

- `exa_combat::lib`; `exa_game::thaum::{shell_hum, effect_light, effect_draw, hold_live, kinesis_live, cast, live, force}`, `exa_game::thaum` (strikes span and line), `exa_game::effects::emissive`.
- Evidence: REA (Ghidra) pseudo-code and the listing of the kinesis pick callback, the throw's grab creator, the force shell's constructor, scene-join callback, render step, hit callback, ripple step and destructor, the cuboid and strikes emissives' program setups, bounds and draws, the strikes aura builder, activation and weapon callbacks, and the damage routine's three calls of the striker's hook, read from `Exanima.exe`; constants read from its image.

## Out of scope

- A dead character's held shell (the throw's grab of a skinned body) has no box light yet: its node box is the body's, which Exarust does not fit.
- A weapon built of several nodes keeps the single-node line and gradient (the original takes its last node's box, radius and 0.5 weights).
- The strikes hook and the Shell power's share one body slot in the original (the last registered wins, either end clears it); Exarust keeps them apart.
- The shell's random draws come from Exarust's effect generators as the shell already took them.
