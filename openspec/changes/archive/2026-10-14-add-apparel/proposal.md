# Proposal

## Why

Exanima's characters are defined by layered apparel and armour (Phase 3–4 of
`docs/PLAN.md`). Bodies now animate; apparel must fit and follow them.

## What Changes

- Attach apparel models from `Characters.rpk` to a body: each apparel node
  named `<bone><layer><name>` goes under the body part with that bone index.
- Resolve run-time materials (`&<model><set><n>`) to their texture sets in
  `Apparel.rpk` (`…d` diffuse, `…n` normal).
- `--outfit a,b,c` for `exarust character` and `exarust play`.

Out of scope: procedural tinting from masks (`…m1`, `…m2`) and item
materials; hiding covered body parts; the item database.

## Capabilities

### New Capabilities
- `apparel`: fitting apparel models to animated bodies with their texture sets.

### Modified Capabilities

## Impact

`crates/exa_character`, `crates/exa_assets` (run-time material names),
`crates/exarust`.
