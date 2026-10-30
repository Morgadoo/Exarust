# Design

## Context

The experience and trade units were mostly ported. This pass filled the
small routines left and the review corrected two of them.

## Goals / Non-Goals

**Goals:**

- Match the technique word, the power division, the hook order, the level
  start, the setting value, the basket move and the quality share.

**Non-Goals:**

- A story session that calls the hooks; the shop window's widgets.

## Decisions

1. `technique_bits` takes the six skills and a per-skill table; skill 1's
   table is empty. Technique slots past six are clamped (the original reads
   on with no bound, which no trained skill reaches).
2. `Conversion::techniques` reports the rebuilt word; the body applies it.
3. `apply_to_basket` keeps ids only: the original stores a transformed row
   value for a taken item, which Exarust's ids already stand for.
4. `ShopWindow::preview_subject` records the item a pick lays the window
   out on; the opener's layout leaves it unset.

## Risks / Trade-offs

- None of the experience rules runs in a story level yet; the hub's shop
  window binds the stock without a list number.

## Migration Plan

No data migration.

## Open Questions

Which arena list numbers the opener passes.
