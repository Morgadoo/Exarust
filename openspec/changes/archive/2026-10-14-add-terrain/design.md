# Design

## Decisions

- **Per-vertex weights, shader layering.** The original bakes each tile into
  colour/normal/gloss/height textures on the GPU from up to 32 materials. Its
  weight maps have one texel per vertex (241 × 241), so evaluating the rules
  per vertex loses nothing. A tile keeps its four heaviest materials, stacked
  in palette order. Their "over" opacities go in the vertex colour and are
  composited in a WESL `ExtendedMaterial` on `StandardMaterial`, so terrain
  gets Bevy's lighting.
- **Rule semantics from the executable, re-expressed.** Kinds index a table
  of per-vertex inputs. The ramp is linear. The noise maps are regenerated
  from their seeds with the same generator (LCG white noise plus four
  wrapping octaves), so painted detail lands where the original puts it.
- **Heights.** Stored values × spacing × 0.1 give centimetres, as the loader
  computes. The palette rules (e.g. a ceiling rule at 2.3–2.6 m) only make
  sense at this scale.
- **Materials attach a frame late.** `spawn_level` only has the model
  loader. Tile meshes spawn at once, so physics builds their colliders with
  the level, and a plugin system adds the terrain material on the next
  update.
- **Unresolved layers count as unpainted.** As in the original, a layer
  naming no node is skipped, so its share joins the leftover weight that
  goes to node 0.
