# Proposal

## Why

The original builds a linked-body item's `_SKIN` when its model is
imported: the skin builder's only callers are the importer's finalise pass
and the content stream's object builder, and nothing at the attach rebuilds
it. A flail in the hand therefore stays skinned over its own nodes. Exarust
skinned placed objects but drew a held flail's pieces rigidly.

## What Changes

- The held weapon's spawn attaches each skinned member's parts over the
  item's node entities (`exa_world::prop_skin::attach`).
- A weapon stain pass over a skinned part carries the part's skin.
- `--weapon` reads `+` as a space, so a grid scene can hold "flail a01".

## Impact

`exa_character`, `exarust`; exa_character README. The grip, the item's
bodies and the swing are unchanged.
