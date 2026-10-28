# Proposal

## Why

The third DeepSeek pass on the tetrahedral rigid body (lane `tetra3`,
docs/PLAN.md Phase 3, Bodies: skeleton, physics, control) added the
plain body's impact sound, its scrape and roll sound sources, and the
route that takes a prop's stored sound slots into the solver, where the
sources step at the head of every body's post-step. The review checked
each rule against the original's instructions: the impact sound matched;
the scrape sources measured their move as a length where the original's
helper returns the squared distance, placed their point by the current
matrix instead of the one the last tick recorded, sat at the root
instead of their own part, worked their smoothing and pitch in single
precision, and played on a NaN speed.

## What Changes

- object-physics: new Plain body impact sound, Scrape sources and Stored
  sound slots requirements.
- Code: the scrape step measures the squared move in cm², reads the
  part's settled (last recorded) matrix, sits at that part's position,
  works the 0.05 scale, the smoothing, the 0.02 compare and the pitch in
  double, and skips bodies without sources; the slot filter is labelled
  as the empty-name stand-in for the original's audio lookup.

## Evidence

The plain body's collision callback, its post-step and its add-to-world
routine in the tetra decompilation pack, the engine's squared-distance
and MINSS helpers in the full listing.

## Out of scope

Playing the events through `exa_audio`, calling the impact sound from
the pair and level impacts, looking slot names up in the audio
environment, a parent body's matrix on a slot point, and the rest of the
TTetraRBD gaps (breakable body links, group-mobility blend, per-root
iteration counts, share recompute, node-clock draw fraction).
