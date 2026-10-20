# Design

## Behaviour

The rules are in [docs/formats/SETTINGS.md](../../../docs/formats/SETTINGS.md). The notable ones:

- The reader ends a value one byte before its line feed, so files with bare LF lose a byte per value; the game's own one-setting save starts one byte before the name, so it joins the line to the one above. Both are reproduced; the settings screen's rewriter (a corrected copy in the hardware configuration) is reproduced too, as `put_setting`.
- The substring search returns a 0-based offset and scans from `start − 1`; the game's rewriter treats offset 0 as "not found". Emulating the search's instruction flow on sample texts settled the base.
- The splice's grow size is `len + new − old`: the decompiler dropped the index register of the `lea`; the instruction bytes show it.
- A positive `GUISize` is kept only on a display 800 pixels tall (as 1); `FPSLimit` goes through whole milliseconds.

## Model

- `settings_ini`: `REGISTRY`, `read_assignments`, `convert`, `Settings` (`apply`, `read`), `parse_key_def`, `put_setting`, `save_one_setting`, `format_int`/`format_bool`/`format_float`, `fps_cap`, `user_folder`, `settings_file`, `store_file`, `open_settings_file`, `save_one_setting_to_file`.
- `settings_store`: `SettingsStore` (`load`, `get`, `set`, `save`), `BINDINGS_PATH`.
- `data_stream`: `DataStream` and the Latin-1/UTF-8 helpers.
- `file_ref`: `FolderTable`, `FileRef`, `FileName`, `with_extension`, `directory_prefixes`, `read_from_folders`, `FormatRegistry`, `read_whole_file`, `write_whole_file`.
- `exarust::user_settings`: loads both files at launch, applies them to the options, sets the GUI size once the primary display is known, inserts the store's bindings and writes the store back after the app exits.

## Choices

- Exarust keeps its own user folder (`EXARUST_USER_DIR`, else the configuration folder and `exarust`). Writing `FirstRun = 0` into the retail game's file would skip its first-run hardware setup.
- A setting the file does not name keeps Exarust's launch default (its 1600 × 900 window, supersampling on, the readout shown), so a missing or empty file changes nothing.
- Exarust has one supersampling level (the original's 4×): any `Supersample` above 0 keeps it on.
- Full screen is Bevy's borderless full screen on the primary display.
- Malformed stores (sizes that do not nest) are errors; reads past a stream are errors; a `FPSLimit` above 1000, which divides by zero in the original, gives no cap.

## Open questions

- The readers of the shading words (`0x2f3`/`0x273` and the level-6 flag) and of `ShadowQuality`'s levels are not traced; Exarust's point shadows stay on/off.
- `Bindings::decode` picks the older shape by the count, the original by the version word, and the original clears slot 21 after reading; left for the bindings' owner.

## Docs

New `docs/formats/SETTINGS.md`; `docs/UI.md`, `docs/PLAYER.md` and the READMEs updated.
