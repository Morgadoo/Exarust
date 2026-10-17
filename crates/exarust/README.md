# `exarust`

The game binary: `exarust [--game <dir>] [options] (play <level> | level
<name> | character [moveset/action] | model <pack> <entry> | texture
<name>)`; the options are listed at the top of `src/main.rs`. It grows into
the full game as `exa_game` gains modes (see `docs/PLAN.md`). The debug
flags (`--debug-bodies`, `--joint-trace`, `--pair-trace`, `--corpse`,
`--fixed-frames`, `--sim-speed`, `--pause-at-tick`, `--pause-on-stretch`,
`--pause-on-hit`, `--pause-after-jet`, `--pause-after-collapse`, `--press`,
`--click`, `--cursor`, `--still-opponent`, `--debug-wound`, `--debug-nav`,
`--weapon-blood`, `--debug-spray`, `--perks`) and keys are in
[`docs/DEBUGGING.md`](../../docs/DEBUGGING.md).
With `--opponent` the fight starts when the level shows: the duellist enters
combat and, in a practice or arena level, the player's combat mode turns on.
Placed characters keep their records' perks (Remise, Feint, …); the player
and the duellist are new records, with none unless `--perks` gives the
player some.
