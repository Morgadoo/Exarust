# Proposal

## Why

Exarust had no user settings: no `.ini`, no `.set`, no write routine for the original's byte stream, and the launcher's only options were command-line flags. The original keeps a text settings file and a binary settings store in the user folder, and every option it saves goes through them.

## What Changes

- `exa_formats::settings_ini`: the registry of the original's 41 named settings with their kinds; the `Name = value` reader exactly as the original splits, trims and converts lines (including the byte it drops before a line feed); the graphics and window settings' defaults and rules (GUI size, quality clamps, supersample cap, frame cap); the `KeyDef` hotkey macros; the settings screen's and the game's one-setting line rewriters (the latter starts one byte early); integer, boolean and fraction formatting; the user folder and file names.
- `exa_formats::settings_store`: the nested `{tag, size}` chunk store with get, set (enclosing sizes kept), the integrity check and the empty-store rule; the key bindings' tag path.
- `exa_formats::data_stream`: the byte stream's writes (size cut at the cursor, 512-byte growth), u8/u16/u32/f32, text lines, length-prefixed strings and Latin-1 as UTF-8.
- `exa_formats::file_ref`: the folder table, file references, entry stem, extension swap, the folders made for a path, the search path, whole-file reads and writes and the format registry.
- The launcher reads `Exanima.ini` (creating it), applies the window size, full screen, vertical sync, supersampling, frame-rate readout and GUI size it names, writes `FirstRun = 0` on a first run as the game does, uses the store's key bindings and writes the store back on exit. Its files live in Exarust's own folder; `--no-settings` skips them.

## Capabilities

### New Capabilities

- `user-settings`: the settings file, the settings store and their use at launch.

### Modified Capabilities

## Impact

- New modules in `exa_formats`; `exarust` gains `user_settings` and `--no-settings`.
- Docs: new `docs/formats/SETTINGS.md` (linked from `docs/formats/README.md`), `docs/UI.md` (the GUI size rule), `docs/PLAYER.md` (where the bindings are stored), the `exa_formats` and `exarust` READMEs.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: the graphics-and-keys settings cluster (registry, reader, boolean test, rewriter and splice, handlers), `RF_StoreSettings` (path, load, get, set, ancestors, descent, save, exit hook), `RF_DataIO` (stream writes and reads, folder table, file references, whole-file read and write, search path, directory creation, format registry), and the callers that register and save settings (launcher, hardware configuration, game main, audio). The substring search and the splice were checked instruction by instruction; the decompiler dropped an index register in the splice's size. No corpus: user files are not shipped.
- Out of scope: the hardware configuration's own handlers and its auto-detection, running `KeyDef` commands (the debug command registry), the settings screen's apply logic, executable resources, the memory blocks' non-heap kinds, and changing `Bindings`' reader to the original's version rule.

## Phase

Phase 8, settings.
