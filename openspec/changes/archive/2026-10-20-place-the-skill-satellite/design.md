# Design

## Behaviour

The stored code is read from the array at `0x1b8` by the caller. Its angle uses `2.5` rather than the ring position's `3.5`.

The node x is `centre + round(unit × cos × a0 × a4) × turn`. The node y is `centre + round(unit × sin × a0) × turn`; the second offset is deliberately absent, as in the original. The centre shifts are the same as the ring: one unit off x while scrolled with a positive page, three units off y on page 0.

The sample angle is `(sample − 1) × 2π × 0.2`. Its cosine times `unit × 18.3` is rounded and added to x; its sine is rounded and subtracted from y. Both are then scaled by the turn.

## Model

`skill_satellite(stored, sample, ring)` returns the point. The ring fields are the same as `SkillRing`, with the stored code supplied separately.

## Open questions

The missing second offset in y is reproduced as read; whether it is intentional is not established.

## Docs

This method is 533 bytes. `TSkillUI` moves from 9% to about 12%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
