## Context

The feet routine lifts an anchored stepping foot by a rule scaled by a lift
scale; the balance routine computes a scale each tick a foot is chosen.

## Decisions

- Port what the original reads: the lower leg's own field, zero for the
  whole game. The balance routine's computed scale has no reader and is not
  kept.

## Open questions

- None for the lift: the image has no writer of the leg's field (all eight
  accesses to the offset listed in docs/re/motiledyn-map.md's entry).

## References

docs/re/motiledyn-map.md (backfall lane).
