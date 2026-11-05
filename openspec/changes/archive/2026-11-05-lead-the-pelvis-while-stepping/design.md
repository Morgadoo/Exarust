## Context

The body tick builds every pin target from the frame, leads the targets
while a step runs, and clamps them around the feet centre; the balance
routine places the stance origin. Exarust had the thorax lead only.

## Decisions

- The pelvis pin is Exarust's root-part point record (its 60·scale clamp).
- The step length the pelvis lead reads is the reach × the legs' extension,
  what the feet routine leaves for a plain step each tick.
- The leads read the frame anchor (`drive.anchor`) as the standing point.

## Open questions

- Whether retail trips at exanima01's spawn on a held S: needs a retail
  capture with scripted keys (not available).
- Exarust keeps two standing points (the step record's and the frame
  anchor); the original one. The jolt and step end write the first.

## References

docs/re/motiledyn-map.md, docs/PLAYER.md (Combat footwork).
