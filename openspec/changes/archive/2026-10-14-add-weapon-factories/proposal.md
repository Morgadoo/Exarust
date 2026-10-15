# Proposal

## Why

Most of the game's weapons are assembled by factory recipes from component
models, chosen by each item's seeds. Exarust could only hold plain models,
and the levels' weapons and the characters' held items were missing.

## What Changes

- `exa_formats::factory`: the recipe format (all versions), the seeded
  generator (selection, deforms, materials) and placement.
- `exa_formats::objdb`: items' weapon section (seeds, condition, stored
  choices).
- `exa_assets::weapon`: assembled weapons from painted, deformed components.
- `Wielded` carries weapon data. Placed characters hold their items, and
  placed `@` objects are assembled.
- `exa-tool corpus factories` with the original's worked examples.

## Capabilities

### New Capabilities

### Modified Capabilities
- `combat`: factory weapons.

## Impact

`crates/exa_formats`, `crates/exa_assets`, `crates/exa_character`,
`crates/exa_world`, `crates/exa_tool`, `crates/exarust`, docs.
