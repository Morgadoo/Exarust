# Design

## Decisions

- **Paint on the CPU at equip time.** The original paints on the GPU with
  its own shader programs. A CPU painter is simpler, testable and runs once per
  item and appearance. A 1024² item takes well under a second at opt-level 3,
  and the result is cached. The finishes re-express the original's rules in
  Exarust's own terms.
- **Appearance by model name.** Worn items are named by their model entry.
  The first object-database item showing that model supplies the appearance
  until characters carry item instances.
- **Library sampling scales** follow the original: weave and impurity maps
  one texel per painted texel, damage, dirt, rust and scuffs at 1024 texels
  across. Some metal texture slots are inferred from names.
