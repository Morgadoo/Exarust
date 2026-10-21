# Proposal

## Why

The settings screen's unit (the original's hardware configuration) was 7% reproduced: only the binding file and the startup key registration existed. Nothing modelled the options' ranges, steps and labels, the three copies of the options the page keeps, Apply, Cancel and Defaults, the GPU benchmark and its quality tiers, or the Controls page's rows and key names.

## What Changes

- `exa_formats::hw_config`: the options, their defaults, slider steps and labels; the page's draft and saved set (opening reads the settings file again, Cancel and the auto-detection push the saved set back); the benchmark's protocol, score and quality tiers, the Deck preset; Apply's lines in order; key names and title-casing; the startup handlers of the hardware settings, `Volume`, `ShowGameTips` and `CheckPoints`.
- `exa_ui::settings`: the Settings rows' ids, what they show when the page opens, where each slider or checkbox change goes, the anti-aliasing row's enabled rule, the aim-speed row, the Controls rows' key cells and binding slots, the form's tab and button actions.
- `exa_game::hw_config`: the live options, saved set and page; open, Apply (with its read-first abort), Cancel, Defaults per page; the benchmark driver with an off-screen 1024² target, a 512² texture and sixteen quads; rebinding the Controls page's copy of the bindings; systems pushing the live options into `exa_render::quality`, the audio, the frame-rate readout and the window. `exa_game::bindings` names its power slots.
- `exa_render::quality` and `supersample::SuperSampling`: shadow-map sizes, MSAA while super-sampling is off, the super-sampling target turned off and on.
- The launcher seeds the options from the settings file with its own flags winning; `--benchmark` runs the auto-detection once.

## Capabilities

### New Capabilities

- `hardware-config`: the settings screen's options, copies, actions, benchmark and Controls rows.

### Modified Capabilities

## Impact

- New modules `exa_formats::hw_config`, `exa_game::hw_config`, `exa_render::quality`; `exa_ui::settings`, `exa_game::bindings`, `exa_render::supersample` and the launcher extended.
- Docs: `docs/UI.md` (Settings options), `docs/formats/SETTINGS.md`, `docs/PLAYER.md` (Bindings), the `exa_formats`, `exa_game`, `exa_render` and `exa_ui` READMEs.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: the unit's 89 functions' listings (every float conversion and comparison read from the instructions), its three control-record tables (row ids, slider steps, handlers, the Controls page's 22 label/key-cell pairs), the startup key handlers and the audio and game handlers of `Volume`, `ShowGameTips` and `CheckPoints`, the vertical-sync setter, the file reader Apply calls first, the saved-to-player key-table conversion and the start-up caller of the first-run auto-detection. No corpus: the options live in user files, which are not shipped.

## Out of scope

Drawing the Settings and Controls forms (the rows' model is ready), the key-read screen, the shader-cache progress screen and the "Configuration" message box, Steam Deck detection and Steam Input, the video-device and video-memory readouts, renderer features for shading quality, texture quality and detail, shadow detail and the vertex-buffer, mip, swap and GI-precision switches, the aim-speed scaling, the first-run auto-detection at launch, and the binding file's scan-code shape (Exarust keeps its virtual-key table).

## Phase

Phase 8 (Polish and release), settings.
