# Design

## Decisions

- The scrape move is the squared distance in centimetres: the original's
  distance helper returns the sum of squares and the curve
  `(√d·2 + d)·0.05` takes it as is. The body solver works in metres, so
  the difference is scaled by 100 before it is squared.
- The point is placed by each part's settled transform, which is the
  matrix the previous post-step recorded; the original's loop runs before
  the tick's matrix is rebuilt and reads the copy kept from the last
  tick. The playing source's position is that matrix's translation for
  the part carrying the slot (one original body is one Exarust part).
- The impact sound keeps its own state struct and is a tested function:
  nothing in Exarust can play it yet, so it is not called from the pair
  or level impacts, which keeps the tick cost unchanged.
- Bodies without sources return from the scrape step at once and the
  events vector is reused, so the per-tick cost for the shipped props
  (none in `Objlib.rpk` carries a slot) is a length check.
