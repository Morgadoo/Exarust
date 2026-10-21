# `exarust`

The game binary: `exarust [--game <dir>] [options] (play <level> | level
<name> | character [moveset/action] | model <pack> <entry> | texture
<name>)`; the options are listed at the top of `src/main.rs`. It grows into
the full game as `exa_game` gains modes (see `docs/PLAN.md`). The debug
flags (`--debug-bodies`, `--joint-trace`, `--pair-trace`, `--corpse`,
`--fixed-frames`, `--sim-speed`, `--pause-at-tick`, `--pause-on-stretch`,
`--pause-on-hit`, `--pause-after-jet`, `--pause-after-collapse`, `--press`,
`--click`, `--cursor`, `--still-opponent`, `--debug-wound`, `--debug-nav`,
`--bevy-sun-cascades`,
`--weapon-blood`, `--debug-spray`, `--perks`) and keys are in
[`docs/DEBUGGING.md`](../../docs/DEBUGGING.md).
`--max-fps <n|auto>` is the original's frame-rate cap (a fixed rate, or
`auto` for the display's rate at least 120 Hz) and is off by default, so no
frame is held; `src/frame_loop.rs` applies it from the previous frame's
start, keeps a background window to winit's 32 ms reactive update and binds
Alt+Enter to the windowed/borderless toggle with its size and position
restored. A launch full screen asks for an exclusive display mode of the
requested size when the monitor lists one, as the original changes the
display mode, and falls back to the current mode where the platform has no
exclusive mode. The window's minimum track size is the original's 640 ×
480. Rules: [`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md) (Frame
pacing).
The user's settings files (`Exanima.ini`, `Exanima.set`,
[`docs/formats/SETTINGS.md`](../../docs/formats/SETTINGS.md)) are read at
start from `EXARUST_USER_DIR`, else `~/.config/exarust`: the window size,
full screen, vertical sync, supersampling, the frame-rate readout and the
GUI size they name apply unless the command line overrides them, and the
store's key bindings are used. `--no-settings` ignores them.
With `--opponent` the fight starts when the level shows: the duellist enters
combat and, in a practice or arena level, the player's combat mode turns on.
Placed characters keep their records' perks (Remise, Feint, …); the player
and the duellist are new records, with none unless `--perks` gives the
player some.
