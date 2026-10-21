# Design

## Behaviour

The rules are in [docs/UI.md](../../../docs/UI.md) (Settings options) and [docs/formats/SETTINGS.md](../../../docs/formats/SETTINGS.md) (Hardware configuration settings, The settings screen's options). The notable ones:

- Three copies: live, draft, saved. Opening takes the saved set from the live options, reads the settings file again over them into the draft and restores the live quality, sampling, vertical sync and window mode from the saved set. Nothing is reset on opening.
- The volume, music, super-sampling and window rows show the live options; the others the draft. Hardware anti-aliasing is disabled (the control's disabled state bit) while super-sampling is on.
- The volume, texture-detail and texture-quality sliders and the vertical-sync, frame-rate, tips and checkpoints checkboxes are live; the others change the draft until Apply. Music mutes the session only.
- Apply reads the file first and does nothing when it cannot; it writes the lines in the original's order from the draft and live sources listed in the docs, commits the bindings copy when its Forward key is set, toggles full screen and pushes super-sampling, shading, shadow quality and shadow detail when they differ from the saved set. Hardware anti-aliasing waits for the next start.
- Defaults depends on the page: the benchmark (or the Deck preset) and a refill on Settings; the shipped keys or the saved aim speed on Controls. The auto-detection keeps the window mode the saved set has and its push skips the live shadow detail and the draft's anti-aliasing and window mode.
- Label scales: the first branches of the Off…Ultra and Coarse… scales compare unsigned. Slider steps and labels go through the processor's to-nearest conversion, 64-bit with the low 32 bits kept.
- Startup handlers: `Volume` × 0.01 in single precision, `AimSpeed` (step − 1) / 99 in double precision unclamped, `InputMode` greater than 0 is the controller.
- Controls: 22 label/key-cell pairs; each key cell's id is one more than its slot in the saved key table, and the table's conversion into the player's table ties the power slots to Power Bar, Power Tree and Use Power.

## Model

- `exa_formats::hw_config`: `Options` (`reread`, `auto_detected`, `deck_preset`, `pushed_live`, `pushed_draft`, fractions and labels), `Quality::auto`, `Scale`, the slider steps, `Screen` (`open`, `cancel`, `quality_change`, `settings_lines`, `write_settings`), `Benchmark`, `key_name`, `title_case`, `apply_setting`, `options_from_settings`.
- `exa_ui::settings`: `Setting`, `populate(live, draft)`, `on_slider`, `on_toggle`, `set_aim_speed`, `BINDINGS`, `form_action`.
- `exa_game::hw_config`: `HardwareConfig` (`open`, `close`, `switch_page`, `apply`, `cancel`, `defaults`, `finish_benchmark`, `deck_preset`, the control calls), `BindingEditor`, `binding_rows`, the plugin's systems.

## Choices

- Exarust's key table holds virtual-key codes, so the key names come from a table by virtual-key code that gives the names the system call gives the original's scan codes.
- The benchmark's clock starts at the first drawn frame (Exarust is frame-driven); the workload is sixteen sprites into the same target, so the score is comparable in spirit only.
- The saved set starts from the launch options, not the original's variables' initial values (which would make a first auto-detection switch to full screen).
- Without a `Supersample` line Exarust's own super-sampling stays on (live level 4); the launcher's flags win over the file.
- The shadow-map sizes per shadow quality are a guess around Exarust's shipped sizes.

## Open questions

- What the shading-quality, shadow-detail and texture settings change in the original renderer: not traced (REA on the renderer's readers of those variables).
- The key-read screen's handling of Escape and of the mouse buttons: outside this unit (REA on the key-read screen's methods).
- The binding file's scan-code shape against Exarust's virtual-key table: needs the player-controls conversion modelled both ways.

## Docs

`docs/UI.md`, `docs/formats/SETTINGS.md`, `docs/PLAYER.md` and the READMEs.
