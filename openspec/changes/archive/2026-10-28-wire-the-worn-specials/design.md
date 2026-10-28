# Design

Extends `docs/formats/OTHER.md` (Armour, the special kind byte).

## Decisions

- The dispatch lives in `exa_formats::encumbrance` beside the encumbrance
  sum, as one value (`WornSpecials`) the spawn computes once per record.
  The effect items stay in one ordered list because the pass attaches them
  in wearing order and the list runs in that order.
- The effect list is a component on the fighter's entity
  (`WornEffects`), holding the body's copy of each item's crystal bytes:
  Exarust's spawned bodies do not write item records back.
- The once-a-second run sits in the fixed tick after the health
  bookkeeping and before the character ticks; the original runs it from
  the body tick right after the focus tick (labelled as a placement guess).
- The mind-masked bit is a marker component read by the brains' and the
  duellist's perception; the existing `perceive` already shrinks the range
  for a hidden character with no masking power (0.9 and 0.8 of the
  observer's darkness).

## Open questions

- What the healing step's HUD flash draws for the player: answered later
  by REA on the HUD's 1000 ms timer when the HUD grows a health flash.
- The order of the effect run against Exarust's character stages: answered
  by a retail capture of red health under a healing mask if the second's
  phase ever matters.
