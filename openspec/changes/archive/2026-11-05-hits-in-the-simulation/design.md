## Context

Follows `rise-in-the-simulation`. No `docs/formats/` page is involved.

## Decisions

- The chain reads the transforms the frame last propagated; at the next frame's first fixed pass they are those of the previous `PostUpdate`, which it read before.
- The frame start runs only on frames that tick, in both worlds; a frame without a tick leaves its blows waiting as messages until the next one.
- A debug pause armed by a blow counts from the tick count of the frame start that took it.

## Risks / Trade-offs

- Since round 69 (footwork through the body) the arena duel's joint trace varies from run to run by more than a degree in main itself (1.57 mean / 24.9 max over the first 100 ticks between two main runs), and one-world digests differ from the first steps, which run while the level still loads. The duel no longer separates a bridge difference from that variation; the scripted walk does (exact behind the bridge).
- Frame presentation (blood, sprays, sounds) takes the chain's results a frame later than before.

## Open questions

- None for reverse engineering.
