## Why

Phase 5 (rendering) in docs/PLAN.md: the "Default material shader
factory" and "RF_Materials (unit routines)" components. Their gaps were
that the factory tables were not wired (the type table and factory list
were tested values) and that the diffuse and opacity layers were bound as
two maps where the original keeps one shared colour slot. The geometry
node's per-part "bound" was also an untraced material value.

## What the original does

REA and the disassembly of the record loader, the layer loads, material
activation, the default builder, the factory registration and the
factory's start-up hook, the texture manager's create call and the
geometry node's activation:

- The loader loads the diffuse layer and the opacity layer into the same
  slot, so the later of the two wins (the opacity layer, in every shipped
  record that has both). The texture manager never returns nothing: a name
  it cannot find gets its grey placeholder.
- The builder binds that slot as the colour of a lit record with the
  diffuse bit, the opacity bit then reading the same image; an unlit record
  with the diffuse bit binds nothing there; without the diffuse bit the
  opacity bit binds the slot alone.
- The start-up hook resets the type table and registers one factory
  record, the default builder (mask 5); no other unit registers a factory
  record. Activation calls every record whose mask meets the record's
  plug-in mask, which is 1 in every shipped record.
- The loader derives a render-state word (the stream needs) and keeps the
  shared colour (specular × level + diffuse); geometry activation copies
  the first into each part's mode word and takes the per-channel maximum of
  the second as the node's bound.

## What changes

- `exa_render::material_state`: `colour_slot`, `colour_binding`,
  `MaterialFactory::startup`, `builds_default`, `record_type`,
  `loaded_render_state`, `late_class`.
- `shader_program::feature_key` and `state_words` read the start-up
  tables; a record whose plug-in mask calls no builder has no state.
- `exa_assets`: the colour slot binds one image as diffuse and opacity.
- `exa_world::geometry_node`: `PartFlags::of_material`; the bound is the
  shared colour, the mode word follows the render-state word.
- `exa-tool corpus materials` lists plug-in masks and records with both
  layers.
