# Proposal

## Why

The sixth DeepSeek pass on the body dynamics class (lane `motiledyn6`,
docs/PLAN.md Phase 3, Bodies: skeleton, physics, control) moved the leg
IK's fold from the aim into the sampler's IK step, kept the stance
score's unordered edges, and had the spawn sample its first clip before
the placement's leg snap, which it now calls. Its review checked each
rule against the original's instructions and found: the sampler's fold
recorded the clip's ankle and hip in the body root's frame while the
feet's lag and both leg IKs read them as world points, so any body away
from the origin or turned would bend and push against the wrong ankle;
the fold left the foot where the clip put it while the original's IK
chain places every part under its parent's; and the stance score left
the feet's height difference in its projection and side terms and kept
an epsilon of 1.2e-7 where the original zeroes that height and compares
against its general epsilon. With the leg snap called at spawn the
standing player still settled 4.5 cm lower (105 cm against 110 cm with
the snap left out, on the RTX 3090 check), so the spawn keeps its
first clip sample and the snap stays uncalled.

## What Changes

- physical-body: the Leg bend and Stance score requirements are modified;
  a new Sampler IK step requirement.
- Code: the IK step records its clip ankle and hip in the chain's frame
  and the aim puts them in the world after its carry; the fold re-places
  the parts under the shin; the stance score zeroes the height difference
  and uses the general epsilon. The spawn samples its first clip; the
  placement's leg snap stays a tested, uncalled rule.

## Evidence

The living sampler's clip chain, IK chain and frame steps, the stance
score routine, the placement routine's sampler call, read as
instructions in the motiledyn decompilation pack and the full
disassembly; a GPU check of standing, walking and a duel against the
previous main build.

## Out of scope

Calling the placement leg snap at spawn and applying its frame height to the controller, the placement
search and scene-node sphere query, the look-point smoothing, the start
capture's upper-body blend, the parry clash point.
