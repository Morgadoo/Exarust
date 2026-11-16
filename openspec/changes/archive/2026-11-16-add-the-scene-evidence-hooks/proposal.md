## Why

The scenes owed by rounds 273 to 280 run on the build worker's GPU, and
several needed input the flags did not reach: a brazier-sized fire for the
fires' body contacts and burns (only the character view's `--torch` stood a
fire), a factory weapon with chosen paint seeds for the generated textures'
picture checks (a fuller, a gem setting), the places of exanima01's sliding
doors and levers to frame a lever-driven gate, a lever turned back to send
its opposite signal, and a scripted story death that is reconstructed (the
death screen's CONTINUE needs a device scan's reconstruction word).

## Evidence

No original function is ported or changed. The debug scan now sets the two
story words the reconstruction device's scan sets (the use's scan branch
writes the reconstruction word and the scanned identity), so a scripted
story death takes the same reconstructed branch.

## What Changes

- `--fire x,y,z[,size]` (play mode): a fire of that size (default 0.8)
  stands at the point from the load.
- `--weapon-seeds a,b[,q]`: a factory `--weapon @name` is built from those
  paint seeds and quality.
- `EXA_MECHANISM_LIST=1`: lists every sliding leaf and lever with its place,
  state and targets once the level stands, then the awake leaves' fractions.
- `--mechanism-poke F1,F2,...`: each frame pokes again (the lever goes back
  to its other detent).
- `--debug-scan`: also sets the story session's reconstruction word and
  checkpoint identity.
- The shipped recipes' overlay and gem seeds are printed by two existing
  tests, for the picture checks.
- A log line's wording: "keeps a checkpoint" / "keeps no checkpoint".

## Impact

Debug aids only: nothing changes without the flags. Crates `exarust`,
`exa_world`, `exa_game`, `exa_assets`.
