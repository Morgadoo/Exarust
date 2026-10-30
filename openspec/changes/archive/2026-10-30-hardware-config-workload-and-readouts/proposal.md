# Proposal

## Why

The hardware configuration unit was about 85% reproduced. The GPU benchmark drew sprites in place of the original's own fragment program. The device and memory readouts and Steam Deck detection were missing. The Controls page had no power-tree help box and no Keyboard/Controller view tabs. The older dialog's slider steps and the shading quality's high flag were not modelled either.

## What Changes

- `exa_render::benchmark`: the original's benchmark fragment program, translated one for one, as a 2D material. It reads a 512² texture whose every byte is 0x3f, with repeat addressing and linear filtering. `exa_game::hw_config` draws it into the 1024² target: four quads a warm-up frame and sixteen a measured frame, all at one depth so the 2D depth test shades each one.
- `exa_game::hw_config`: `steam_deck` (the Steam runtime's marker or the DMI product name), `video_memory_mb` (the DRM cards' `mem_info_vram_total`), `run_detection`, the controller view (`steam_input`, `controller_view`, `set_controller_view`). Defaults tests the controller view, not the bare controller mode. A page switch measures nothing and keeps the page's key copy.
- `exa_ui::settings`: the power-tree help text and its box (2, 48 units; the client less 4 units wide; 50 units high), drawn by `exa_game::settings_form`. Also the Keyboard/Controller tab actions and their placement (21 units with the tabs), and the device and memory readouts.
- `exa_formats::hw_config::legacy`: the older record table's six slider steps. `exa_render::quality::shading_high`: the shading setter's `> 5` flag.

## Evidence

REA/Ghidra and the objdump listing of `Exanima.exe`:
- The benchmark routine: its texture uploads (RGB8, 1024² and 512², the 0x3f fill, repeat and linear parameters), the always-pass depth function, the four-draw warm-up loop and the sixteen-draw measured loop.
- The program text, read from the binary.
- The form constructor: the readouts, "Valve Steam Deck", "Unknown", the help box's 8, 192, client − 16, 200 at unit 4, and its text, read from the binary.
- The page switch and the view-tab handlers (the record state bits).
- The handler that runs the detection or copies the shipped scan-code table (0x11, 0x1f, 0x1e, 0x20, …). It has no direct caller, and Exarust already had it as Defaults. The page switch does not call it.
- The six older slider callbacks (`MULSS`, `CVTSS2SI`, then the add or the shift).
- The shading setter's `CMP 6 / JL`.

## Out of scope

- The benchmark's "Configuration" message box.
- The controller view's own four-record page.
- Steam Input and the Steam callback (platform glue).
- A high-quality shading path and per-detail shadows.
- The exclusive full-screen mode switch.
- The dialog that uses the older record table.

## Phase

docs/PLAN.md Phase 8 (Polish and release), settings.
