# `exarust`

The game binary: `exarust [--game <dir>] [options] (play <level> | level
<name> | character [moveset/action] | model <pack> <entry> | texture
<name>)`; the options are listed at the top of `src/main.rs`. It grows into
the full game as `exa_game` gains modes (see `docs/PLAN.md`). The debug
flags (`--debug-bodies`, `--joint-trace`, `--pair-trace`, `--corpse`,
`--fixed-frames`, `--sim-speed`, `--pause-at-tick`, `--pause-on-stretch`,
`--pause-on-hit`, `--pause-after-jet`, `--pause-after-collapse`, `--press`,
`--click`, `--cursor`, `--still-opponent`, `--arena-match`, `--debug-wound`, `--debug-nav`,
`--bevy-sun-cascades`,
`--weapon-blood`, `--debug-spray`, `--perks`) and keys are in
[`docs/DEBUGGING.md`](../../docs/DEBUGGING.md).
`--max-fps <n|auto>` is the original's frame-rate cap (a rate, or `auto`
for the display's rate) and, like the settings file's `FPSLimit`, is off
unless given, so no frame is held; `src/frame_loop.rs` applies it from the
previous frame's start, releases the held keys and buttons when the focus
changes and binds Alt+Enter to the windowed/full-screen toggle with the
framed window's size and position restored. A background window runs on
winit's low-power 32 ms update. A launch full screen is a borderless window
over the primary display; the settings' `ScreenWidth`/`ScreenHeight` can
ask for an exclusive display mode instead (`exa_game::hw_config`). The
window's minimum size is the original's 640 × 480. Rules:
[`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md) (Frame pacing).
The user's settings files (`Exanima.ini`, `Exanima.set`,
[`docs/formats/SETTINGS.md`](../../docs/formats/SETTINGS.md)) are read at
start from `EXARUST_USER_DIR`, else `~/.config/exarust`: the window size,
full screen, vertical sync, supersampling, the frame-rate readout and the
GUI size they name apply unless the command line overrides them, and the
store's key bindings are used. `--no-settings` ignores them.
With `--opponent` the fight starts when the level shows: the duellist enters
combat and, in a practice or arena level, the player's combat mode turns on.
In an arena level the match is ticked as the original ticks a career match
(its first second, then who stands), and the music is the arena track the
original's match start draws, at its own volume. The arena hub plays the hub track the
original's hub entry draws (Msc_Hub01 two times in three), at 0.1, from one of
its four start places.
Placed characters keep their records' perks (Remise, Feint, …); the player
and the duellist are new records, with none unless `--perks` gives the
player some.
