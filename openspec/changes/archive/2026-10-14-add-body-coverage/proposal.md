# Proposal

## Why

Skin showed through apparel: the original hides body parts covered by worn
items, by per-slot flags in each item. Some apparel also stayed unpainted
because its texture stem was derived from the material name the wrong way.

## What Changes

- `exa_formats::objdb`: the per-slot layer table of items and the slots
  they hide.
- `exa_character`: garments carry the slots they hide. Covered body
  parts' meshes are hidden when a character is dressed.
- Apparel texture stems are the ten characters after `&`, as in the
  original.

## Capabilities

### New Capabilities

### Modified Capabilities
- `apparel`: body coverage.

## Impact

`crates/exa_formats`, `crates/exa_assets`, `crates/exa_character`,
`crates/exa_world`, `crates/exarust`, docs.
