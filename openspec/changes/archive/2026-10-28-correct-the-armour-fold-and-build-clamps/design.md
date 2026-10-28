# Design

- The fold keeps the listing's operand grouping. Two-operand sums are
  commutative in IEEE arithmetic, so only the product grouping matters:
  (1 − s0′)·(s1·γ1) differs from ((1 − s0′)·s1)·γ1 in the last bit for
  some inputs; a three-item test pins one.
- The original's slot-walk stack is a fixed buffer; past 31 entries it
  logs and overruns it. Exarust's stack grows, logs the same message and
  walks on, which is the original's behaviour without the overrun.
- The record clamps are applied where the values are read instead of
  mutating the parsed record, so the saved record stays as loaded.
- The second wind is resolved with the same item lookup as the
  encumbrance, so an id that finds no item grants nothing.
