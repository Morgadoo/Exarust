# Proposal

## Why

Running switches the whole clip to Run. The original keeps Walk playing and slerps each bone toward the Run pose by 0.7 times the run blend. That is the next piece of Phase 3 gait walking (`docs/PLAN.md` §7, milestone M16) after the standing leg bend.

## What Changes

- Forward locomotion plays Walk whether the player walks or runs. The run blend no longer selects the Run clip.
- While Walk plays and the run blend is above zero, each bone is the Walk sample slerped toward the Run sample at the same keys by 0.7 times the run blend. The overlay is applied before the pose hold. The root stays the Walk sample.
- **BREAKING** for the played clip name: a running player is on Walk, not Run. The jog clock rate is unchanged, because that rate already follows the run blend rather than the clip name.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `player-movement`: forward movement, walking or running, plays the walk clip.
- `motion-playback`: the run overlay on that walk, 0.7 times the run blend, root unchanged.

## Impact

- `exa_game` player steering and the living sampler in `limp`.
- `exa_anim` samples an optional second clip at the same keys before the pose hold.
- `docs/PLAN.md` and `docs/formats/RSQ.md` (Motion playback).
- Evidence: the animation notes' locomotion clip choice and the living sampler's overlay step, already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: the sprint overlay weight (the full run blend, once sprint exists), playing Walk backwards instead of the Backpedal clip, the locomotion start key, stride modulation, gait plants, footsteps, and the locomoting leg bend.
