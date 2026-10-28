# Proposal

## Why

docs/PLAN.md, Phase 3 (Bodies: skeleton, physics, control), the rigid-body
milestone, with its sounds under Phase 5 (Visual and audio fidelity): the
tetrahedral rigid body's review left its collision sound and scrape loops
unplayed for props, the step's frozen branch absent and the per-root
link-iteration count unstored.

## What Changes

- A prop's parts carry their node's first mesh material's collision sound
  set, and each stored sound slot keeps its sound name.
- Each body tick, the pair impacts and then the level impacts on a prop
  ring the plain body's collision call on the part's own clock and
  threshold and play the set as a one-shot at the part's mesh centre.
- Each scrape source drives one attached loop: started when silent over the
  speed floor, following the source's volume, pitch and point, stopped
  under it, freed with the body.
- A frozen body's step holds it on its previous particles and takes one
  tick off its countdown (no shipped path freezes a body).
- Each root keeps the original's link-iteration count (2 plus one per
  articulated child, 20 living, 10 dead) and its count-reached mark; the
  solve's 20 passes are unchanged.

## Evidence

REA/Ghidra and the disassembly of Exanima.exe: the plain body's collision
call, set-up, post-step, step and the link-iteration routine, registration
and the death path; the installed `Sound.rpk` (the `wdlcol`, `mtlcol` and
`stncol` sets).

## Out of scope

The group link-mobility blend (it needs the link and contact mobility
split), removing a broken grab with its parent's chain, the dormant
statue's count of 5, the cursor's kinematic hold, and running the volume
fit in a live builder.
