# Proposal

## Why

The character generator's dressing pipeline had its garment assembly but
not the list the skin builder appends, nor the per-slot impact material,
the spawn plans, the attach pass, the feral underwear overlay or the
background rebuild's apply step. This is Phase 3 (bodies) and Phase 5
(visual fidelity, the character surface) work on `SG_CharacterGen`.

## What Changes

- `exa_character::dress`: the dressed mesh's append list. Each garment (in
  layer-level order) appends its chosen nodes in slot order 1 to 18, gated
  by the slot's select mask against the garment's visible mask and with
  the slot's cull bits; the body's visible slots follow in slot order; the
  overlays last. The body-only branch (race look bit `0x4` clear) appends
  every body slot ungated.
- `exa_character::rebuild::dress_plan`: garments ordered by layer level
  (stable), and the per-slot impact material of the garment that leaves
  the largest uncovered word (unsigned 64-bit compare, first wins).
- `exa_character::rebuild`: the feral underwear overlay
  (`(2 − f)·f·255`, nearest-even, on damage and dirt), the re-texture
  layer, the preview and texture-only wrappers, and the apply plan.
- `exa_character::spawn`: placed, player and corpse spawn plans, the
  creator races' roles, the `&` and `*` material passes, the item model's
  owner word, and the attach plan with the flipped grip.
- `exa_character::runtime::Body::collect_merge_nodes`; `exa_assets::merge`
  carries cull bits and occlusion alpha on the merged mesh.

Review corrections: the append order is slot order, not the slot tree; the
feral bytes are damage and dirt, not quality and damage; the attach pass
hangs one off-hand item (shield, else torch, else second weapon) and reads
the lit flag on weapons only; the apply step clears one texture mark and
swaps only the head's hair; a duplicate of the scene detach (`host`) was
removed.

## Evidence

The generator's assembly: the dressed-mesh routine's garment and body
append loops, the slot-tree weight pass, the merge collection callback,
the attach and equip passes, the three spawns, the re-texture pass and
the apply step, read instruction by instruction.

## Out of scope

Drawing the merged mesh (the game still draws a mesh per node), the skin
compositor, hair as cloth, off-hand items in the running game, applying
saved poses (the corpse-pose lane), the armour pass's special item kinds
and the creator UI.

## Capabilities

### Modified Capabilities

- `character-generator`: the dressed mesh's order, impact material, feral
  overlay, attach pass, spawns and apply step.
