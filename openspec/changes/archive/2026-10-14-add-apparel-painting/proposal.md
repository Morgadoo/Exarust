# Proposal

## Why

Dressed characters rendered grey: the original paints every apparel item's
colour texture at run time from its shade map, its masks, a finish per layer
and the item's appearance in the object database. The textures we drew are
only the shade maps.

## What Changes

- `exa_formats::objdb`: item records (name, model, appearance block). The
  `.rdb` reader now skips table entries that point outside the file.
- `exa_assets::paint`: the palette and the cloth, velvet, leather, suede, mail,
  plate and painted-steel finishes, stacking the layers over their masks.
- `ModelLoader::painted_apparel` caches painted materials per texture stem
  and appearance, with mip chains.
- `exa_character` paints the run-time `&` materials of worn apparel from the
  appearance of the item that shows the model.
- `exa-tool paint <model> <stem> out.png`.

Out of scope: the normal and gloss/metal paint passes, wood, bronze, gem and
resin finishes, pattern textures, per-instance wear.

## Capabilities

### New Capabilities

### Modified Capabilities
- `apparel`: painted colours.

## Impact

`crates/exa_formats`, `crates/exa_assets`, `crates/exa_character`,
`crates/exa_tool` (now depends on `exa_assets`), `Cargo.toml` (dev opt-level
for `exa_assets`/`exa_world`), docs.
