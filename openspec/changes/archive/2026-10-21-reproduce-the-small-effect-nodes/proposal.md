# Proposal

## Why

The original's small effect nodes (`TFireEffectRow`, `TScanEffect`, `TBubbleEffect`, `TShellEffect`, `TDomainWheel` and the level portal routines of `SG_Dynamics_Effects`, 32.5 KB together) give the game its lit blades, teleport bubbles, scans, displacement shells, the power screen's domain wheel and the level portals' sparks and light. A first pass reproduced their rules; a review against the instructions found the wheel's icons on the wrong axes, the shell's fade, the bubble's shrinking fade, the flame lag's axis, the dissolve's jitter and push, and a set of single- versus double-precision steps that differ from the original.

## What Changes

- The effects run on the original's game clock in whole milliseconds; spans are signed and deadlines do not wrap.
- The domain wheel places icon 1 at the top of its ring and the others clockwise, names its third domain Body, and has its label, open ease, choice flash and confirm sound.
- The displacement shell's fade is (1 − shown²)²; the shell and scan share a fade plane on the body's up axis.
- The teleport bubble uploads its own value as the fade while it shrinks; its swell and texture period follow the original's integer and double steps.
- The flame row leans its lag 20 cm up the y axis, takes its fades, walks, light and sound in double precision, starts at rest on its item and leaves the scene when it fades out.
- The portal dissolve jitters and pushes each particle's position out from the body's centre; the dissolve system is `_PrtlDisp`.
- A loaded level portal marker (the `0x3DD0E004` emitter node of `uwportal01`) gets the portal's blue light.

## Capabilities

### New Capabilities

- `effect-nodes`: the small effect nodes' rules and the level portal's light.

### Modified Capabilities

## Impact

- `exa_game::effects` (`mod`, `wheel`, `shell`, `scan`, `bubble`, `fire_row`, `portal`); `exa_formats::rfc::PORTAL`; `exa_assets::ModelNode::portal`; `exa_world::PortalMarker`.
- `docs/formats/RFC.md` names the portal emitter; the crate README describes the rules.
- Evidence: REA (Ghidra) assembly and constants of every function in the group, the engine's sine-cosine, distance, lerp, add-scaled and maximum helpers, and a scan of the installed packs (one `0x3DD0E004` record, `_PortalPrt` in `uwportal01`, size 100).

## Out of scope

- Drawing: the flame ribbon's vertices, the shell and scan mesh copies' vertex fetch, the bubble's polyhedron and draw, the wheel's programs and geometry, the GL programs of all six shaders.
- The portal's `TParticleSystem` simulation, audio objects and environments, the bubble's GI light values, destructors and RTTI.
- Starting the shell, scan, bubble, wheel and flame row: their callers (displacement powers, teleporters, the portal screen, the power tree, a burning item's ability record) are not in Exarust.

## Phase

Phase 5, visual and audio fidelity: the small effect nodes.
