# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the inventory and character-view
paragraphs: the character view's equipment routines (the doll's attachment
plan, apply step, take-in, take-out, shield take, swap, redress and job
finalisers) and the view's pointer handlers (hover pick, drag start, drop
and release on the figure) were only partly reproduced, and a weapon-set
swap changed the record but never the hands the moveset reads.

## What Changes

- `exa_items::doll`: the attachment plan over the record's two sets, its
  apply order with the stance restart (blend 20, skipped in the swap
  state), the split begin/exchange swap, the take-out by where the item
  sits (a weapon's entry found in either hand, the off hand's light or
  shield clearing the off entry unread, the scheduled promotion only after
  the body's first weapon), the take-in branch order and the moved light's
  flags, the shield take, the redress from the held models, the model
  marks and the three finalisers. `Possessions::record_release` is replaced
  by the doll's take-out.
- `exa_ui::character_view`: the hover pick, the drag start's refunded
  mind cost, the drop onto the figure, the release on the figure, the
  doll scene's settings, facing and frame counter.
- `exa_ui::game_gui::paint_preview_bars` paints the doll panel's preview
  bars with the HUD's bar painter (`hud::paint_bars`).
- `exa_game::equip` wires a weapon-set swap into the `Hands` component the
  moveset and the player light read, keeping Exarust's carried torch.
- Review fixes against the assembly: the take-in's "permission" test was a
  wait for the item's node to load and is removed; the promotion follows
  the first attached weapon, not any main-hand item; the shield take's
  take-out made room for the old item and did nothing; dropped apparel is
  never handed to the shown pack and charges its bulk even when locked,
  at the wear-room cost and without needing a shown character; a locked
  flagged drag is released with answer -1 rather than charging; releasing
  the shown item sets the mark again with no rebuild; the drag start reads
  the weapon section first.

## Evidence

The character view's equipment cluster and TCharacterView routines in the
`itemgui` decompilation pack, checked against the instruction listing (the
drop's weapon cost subtracts the displaced value, then doubles, in single
precision).

## Out of scope

The doll's off-screen render, scene, camera and background quads; item
scene nodes, weapon and shield parts, physics attachment; the one-step
deferral of the live swap (the running swap still trades the sets in one
frame); the inventory and character-screen forms that would call the drop
and release plans; FPC string and unit finalisers.
