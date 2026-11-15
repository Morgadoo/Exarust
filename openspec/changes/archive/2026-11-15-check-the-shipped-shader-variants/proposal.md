## Why

Roadmap tasks B1 and B2 for the shader template family (decision DEC-8).
The original builds every GL program from `shading.flb`'s templates (the
library reader, the expander, the program cache keyed by feature words,
the 29-variant warm-up); Exarust draws with embedded WESL. DEC-8 closes the
template machinery as engine-replaced on one condition: every program
variant the original builds for 0.9.5's shipped data has a WESL path that
gives the same result, and the variants that do not are listed.

## Evidence

`exa-tool shader --variants` walks every material record a draw reaches
(the libraries, then each chunk tree's own records, as the model loader
resolves them) and groups them by program set and key words: 147 groups,
7533 distinct expanded programs at the shipped settings over the level
settings the game ships. The comparison of each group's template flags
against `exa.wesl` found, besides the known missing refraction:

- 49 foliage records (82 draws) select the translucent program
  (`shadetrl_f`, class 7) and were drawn with the PBR branch;
- 56 records (92 draws) set `Shd.FastLgt` and take the fast light loop in
  the PBR program, drawn with the plain loop;
- the shadow and GI taps were fixed at 8 a frame; the original reads
  `SdwSmps` of its 32-entry table (read from the executable),
  and its ambient include indexes the sub-frame's GI taps `PassIdx + j`;
- the lit programs read the opacity map squared only without a diffuse
  map; with the shared colour slot Exarust squared the diffuse alpha.

## What Changes

- `exa_assets::model::record_path`: the WESL branch a record takes, from
  its words; the material build starts from it. New `flags2` bits
  `TRANSLUCENT` and `FAST_LIGHT`, and `exa.wesl` branches for both
  (`shading.wesl`'s `light_translucent`, and `light_fast` for `FastLgt`).
- The shadow and GI taps follow `SdwSmps` (`lights::sub_frame_taps`),
  carried in the light buffer's first element; `SHADOW_TAPS` holds all 32.
- The diffuse map's alpha is kept unsquared when the opacity map is the
  same image.
- `exa-tool shader --variants` lists the groups with the WESL path.

## Impact

Crates `exa_render`, `exa_assets`, `exa_tool`. Pictures change for the
shipped foliage (backlit leaves, a redder lit colour), fast-light plants,
and cut-out or blended records with both colour layers. At the shipped
shadow quality and super-sampling the tap count is unchanged (8 a frame,
2 a sub-frame); the GI's sub-frame window moves.
