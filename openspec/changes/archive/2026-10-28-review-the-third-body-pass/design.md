# Design

Rules: [`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (balance,
footwork, walking ground, samplers).

## Decisions

- The footwork reach is the character's step reach
  (`exa_formats::weapon_record::step_reach`), not a second copy of the
  formula; its product order follows the original's.
- The walking ground difference is read from last tick's grounds, as the
  original reads the legs' stored grounds before its leg loop updates
  them, and it is updated every tick; only a body moving under its own
  power blends by phase.
- The hold and the capture keep the same seven bones: the capture tests
  the 1-based skin bone, the hold the 0-based pose slot.
- The stance score's side axis is the character frame's first axis. Under
  a mirror conversion of the world this keeps the dot of the feet's span
  with it, so it is the original's term; a frame-less test body falls
  back to +X.
- Two feet that mirror each other can score alike to the last bit; the
  test for that case accepts either foot.

## Open reverse-engineering questions

- The foot velocity the early plant reads is taken as the contact point's
  movement.
- The weights' override for a leg with no ground (weight 0, the other 1)
  is not modelled.
