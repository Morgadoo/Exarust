# Design

## Context

Forward movement currently picks the Walk, Run or Backpedal clip. The clock already blends the walk rate toward the jog rate by the square root of the run blend, for either of the first two clips. The living sampler slerps one clip, then holds the pose. The run pose is a second clip at the same key indices, applied between those two steps.

## Goals / Non-Goals

**Goals:**

- A running player plays Walk.
- Each bone moves 0.7 of the way from that Walk sample to the Run sample at a full run, and stays on Walk when the run blend is 0.
- The root and the jog clock rate stay as they are.

**Non-Goals:**

- Sprint, whose overlay weight is the run blend itself rather than 0.7 times it.
- Replacing the Backpedal clip with Walk played backwards.
- The start key chosen on entering locomotion, stride modulation, gait plants, footsteps, and the locomoting leg bend.

## Decisions

- The overlay weight is `0.7 × run`, clamped to 0..=1. Sprint is not a state yet, so the sprint weight is not selected.
- The second sample uses the same source and target key indices and the same blend fraction as the walk. During a capture the walk still starts from the capture; the run sample uses the run clip's keys at those indices.
- A missing Run clip, or a run blend of 0, leaves the walk sample unchanged.
- The pose hold still runs after the overlay, so a full hold replaces it.
- The steering chooses the clip. The sampler only overlays when the playing clip is Walk.

## Risks / Trade-offs

- A run still looks like a sped-up walk until the overlay is on, and like a 70% run at full blend. That is the original's weight, not a full switch to the Run clip, so the silhouette changes less than a clip swap at the old 0.5 threshold.
- Backpedal remains its own clip. The original does not play it. Left for the gait slice that replaces it.

## Open Questions

None that change this slice. The sprint weight and the backpedal clock are named non-goals above.
