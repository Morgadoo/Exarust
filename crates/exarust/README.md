# `exarust`

The game binary: `exarust [--game <dir>] [options] (play <level> | level
<name> | character [moveset/action] | model <pack> <entry> | texture
<name>)`; the options are listed at the top of `src/main.rs`. It grows into
the full game as `exa_game` gains modes (see `docs/PLAN.md`). The debug
flags (`--debug-bodies`, `--joint-trace`, `--sim-speed`, `--pause-at-tick`,
`--pause-on-stretch`, `--press`, `--cursor`) and keys are in
[`docs/DEBUGGING.md`](../../docs/DEBUGGING.md).
