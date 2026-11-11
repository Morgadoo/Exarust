# `exarust`

The game binary: `exarust [--game <dir>] [options] (play <level> | level
<name> | character [moveset/action] | model <pack> <entry> | texture
<name>)`; the options are listed at the top of `src/main.rs`.
`exarust grid <scene>…` (`src/grid.rs`) opens one window split into four
quadrants, each a separate game in a child process started with
`--offscreen <file>` (`src/offscreen.rs`): no OS window, its cameras
drawing into an image that is read back each frame into a shared-memory
file, and input arriving on standard input. The debug screenshots
(`--screenshot`, `--searcher-shots`) save that image there
(`exa_game::debug_shot`), since the child has no window to shoot. The window's four cameras each
have a viewport a quarter of its size, recomputed on resize, as in Bevy's
split-screen example. The host launches the games one at a time against
`nvidia-smi`'s free memory and forwards input to the quadrant last
clicked. `exarust grid-swap <n> <scene>` replaces one quadrant's game and
`exarust grid-shot <file>` saves the composed window; `--exit-when-done`
ends the host once every scene's game has exited. A scene's leading
`exe=<path>` runs that build in its quadrant and `env:<KEY>=<value>` sets
its environment, to compare two builds or settings under the same load
([`docs/PLAYER.md`](../../docs/PLAYER.md), Unattended runs). The host
plays no sound: it leaves Bevy's audio plugin out, so a stalled host frame
cannot break an output stream into a log-filling error loop.
`--perf-log` (`src/profiler.rs`) adds to each report the assets added or
changed since the last one (`assets:`, a steady stream after the load is a
re-upload loop) and the speed of a write into a fresh upload buffer
(`staging write:`). Its `brains` line is the brains' tick's time and runs
since the last frame, the difference of `exa_ai::BrainTickClock`'s totals:
the clock is the fixed step's, a snapshot for the frame (lent before, it
stayed in the free-running simulation and the line read zero). The workspace patches gpu-allocator
([`patches/gpu-allocator/EXARUST.md`](../../patches/gpu-allocator/EXARUST.md))
so those buffers stay out of a small BAR window: several games sharing the
GPU no longer leave one of them writing its uploads at 0.5 GB/s. It also patches rodio
([`patches/rodio/EXARUST.md`](../../patches/rodio/EXARUST.md)): its
resampler's u32 products wrapped for a pitched voice and sent the audio thread
through 2^32 frames, and the output stream's drop at exit waited 80-125 s for
it (`EXA_EXIT_TRACE=1` traces the exit, `exa_physics::exit_trace`). It grows into
the full game as `exa_game` gains modes (see `docs/PLAN.md`). `--frames n`
without `--screenshot` ends a run after n frames from the end of the load
(a grid quadrant's game then exits). The debug
flags (`--debug-bodies`, `--joint-trace`, `--pair-trace`, `--corpse`,
`--fixed-frames`, `--sim-speed`, `--pause-at-tick`, `--pause-on-stretch`,
`--pause-on-hit`, `--pause-after-jet`, `--pause-after-collapse`, `--press`,
`--pad-press`,
`--click`, `--level-change`, `--cursor`, `--searcher`, `--searcher-shots`, `--still-opponent`, `--opponent-race`, `--arena-match`, `--arena-level`,
`--arena-tier`, `--arena-weapons`, `--debug-wound`, `--debug-nav`,
`--bevy-sun-cascades`, `--floor-torch` (with an optional frame),
`--weapon-blood`, `--debug-spray`, `--perks`) and keys are in
[`docs/DEBUGGING.md`](../../docs/DEBUGGING.md). `--versus` sets the versus
lobby's flag, which Exanima 0.9.5 never sets, so the arena page shows VERSUS;
a `--press` letter or digit is also typed (a scripted run can name a
character in the creator once its name field is clicked).
A Spirit Form cast asks the viewer for its spirit body
(`exa_game::mind::runtime::SpiritBodyAsked`): `spawn_spirit_bodies` builds a
record of race byte 14 through the placed-character path and marks the
body as the caster's spirit, which the mind runtime then hands the player.
The placed-character path builds every race physical and logs the bodies built per race (`bodies built per race: …`); `--spawn-race n@x,y,z` places a living character of race byte n (the first actor template of the race, else the first story level's record of it); it builds the wraith physical with its race's hover
flag, so the spirit floats on its hovering lift and walks.
The game adds `exa_physics::sim_app`'s shell: the simulation's own world,
stepped in lockstep with the fixed step on its `exa-sim` thread, or on the
frame's thread with `--sim-inline`. By default the game also
adds `exa_physics::sim_bridge`: the fixed step's schedules, the character
tick and the brain chain run in the simulation's world, every type they
touch copied across each step (2.3 ms a step in exanima01);
`--no-sim-bridge` keeps them in the frame's world. `--sim-digest <file>`
hashes every bridged type after each fixed step, to compare the two tick by
tick; `--hold-loading` holds game time while the loading screen shows. Under
`--fixed-frames` a grid quadrant drops the host's forwarded input, so a
scripted run repeats whatever the host's mouse does, and the startup
schedules run their systems one at a time (a spawn takes its id when it is
queued, so the GUI's nodes, the camera and the level's root would otherwise
be numbered in thread order).
`--sim-boundary <file>` (`src/sim_boundary.rs`) writes, once the level has loaded and the fixed step
has run 200 ticks, every component and resource type the fixed step's
schedules share with the frame's, by direction, with each side's readers and
writers and the whole-world systems: the hand-off the fixed step's own thread
has to carry. Its "Bridge coverage" section lists the simulation-side types
(query filters included) that no plugin has registered with the bridge yet.
It reads the frame's schedules, so take it with `--no-sim-bridge`.
`--max-fps <n|auto>` is the original's frame-rate cap (a rate, or `auto`
for the display's rate) and, like the settings file's `FPSLimit`, is off
unless given, so no frame is held; `src/frame_loop.rs` applies it from the
previous frame's start, releases the held keys and buttons when the focus
changes and binds Alt+Enter to the windowed/full-screen toggle with the
framed window's size and position restored, and minimises a full-screen
window that loses the focus (not in unattended runs). `src/message_box.rs`
shows the original's modal message box through `zenity` or `kdialog`: the
level-6 "minimum requirements" box when Bevy's renderer finds no usable
graphics adapter, and every level-6 or level-7 message of the engine's
message hook (`exa_game::engine_log`), whose log file the binary names
at start-up: `<project>.log` in the user folder, unless `--no-settings`. A background window runs on
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
store's key bindings are used. `--no-settings` ignores them. The store is
written back on exit (`SettingsStore::save_to`), and a store with no
contents is not written. The flow's own settings writes — the expert-ladder
unlock a novice practice win records, and the story-background unlock the
plot events would record — go through `user_settings::write_flow_settings`,
which writes and flushes the entry as the original's settings writer does.
With `--opponent` the fight starts when the level shows: the duellist enters
combat and, in a practice or arena level, the player's combat mode turns on.
In an arena level the match is ticked as the original ticks a career match
(its first second, then who stands), and the music is the arena track the
original's match start draws, at its own volume. The round's fighters are
the scene body generator's (`exa_game::scene_bodies`): each opponent, the
Beast and the company's other entrants is built from its spawn plan, so the
player's own entry is the only body the play start makes. A career match
started from the hub carries its rank and gear tier into the layout and
holds, on each opponent, the weapon whose object id the match's event drew;
the career itself stays in the running `ArenaCareer`
resource and the match's setup in `PendingArenaLaunch`: the launch is an
in-process level change (`exa_world::level_lifecycle`).
The end screen's Continue is Enter or Space until the screen is drawn: a
next round stands the winner with the reserve or a fresh opponent, and the
end of the match settles the career and changes back to the hub in process,
which places the manager at the return's 300 cm, half turn placement. The arena hub plays
the hub track the original's hub entry draws (Msc_Hub01 two times in three),
at 0.1, from one of its four start places, and asks for the shopkeeper and
one body per employed specialist at their spots.
Placed characters keep their records' perks (Remise, Feint, …); the player
and the duellist are new records, with none unless `--perks` gives the
player some.
