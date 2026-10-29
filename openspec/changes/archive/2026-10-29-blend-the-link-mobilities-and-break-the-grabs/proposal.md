# Proposal

## Why

docs/PLAN.md, Phase 3 (Bodies: skeleton, physics, control), the rigid-body
milestone: the tetrahedral rigid body's reviews left the group link-mobility
blend out (it needed the link and contact mobility split) and the breakable
grab inexact (equal shares, no struck-body disable, the re-anchor before the
gap test, metres in its strain law) with no removal of the broken grab's
chain.

## What Changes

- Every part carries its link mobility beside its contact mobility: a
  limb's is 0.175 of its inverse mass, a rigid body's its registered
  mobility.
- Building an articulated prop's group masks blends each child's link
  mobility with its parent's, once per pass, a pass for every body with
  children; link shares stay as taken before; soft welds read the blended
  values when they run.
- A tested recompute takes every enabled body link's shares from the link
  mobilities as they stand.
- The breakable grab follows the original's order, centimetres, shares and
  NaN compares; after a break every breakable grab to that part is dropped
  with its grab layer bits.
- The node-clock render fraction and a body's draw fraction are tested
  functions; drawing keeps Bevy's fixed-step overstep.

## Evidence

REA/Ghidra and the disassembly of Exanima.exe: the group-mask builder and
its registration caller, the link-share recompute and the character init
that calls it, the plain link iteration's breakable branch and clean-up,
the bite's link builder and the share routine.

## Out of scope

A disabled body state (no Exarust body is disabled, so the struck-body
disable is a parameter), wiring bites into the live combat path, clone
sharing, and calling the node-clock fraction from drawing.
