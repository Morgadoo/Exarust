## Why

Round 28's TMotileDynamics audit (`docs/re/motiledyn-map.md`) listed the character init's race and class presets, the body tick's grip relax, weapon flip and block stamina, and the balance routine's foot sounds as missing. The presets change how the non-human races balance and relax their arms and how loud every character's steps are.

## What Changes

- `CharacterStats::derive` applies the init's race presets as a table over the race record's values: balance bonus (Fael and ghouls 2.0, gobbler 1.5), arm relaxation (Fael 0, hulking ghoul 0.5, golems 0.5, the golem boss 0), footstep volume (the body scale, doubled over 1.3; the ogre's 5) and the golem flag (golem footstep set).
- The spawn feeds the balance bonus and arm relaxation into the body's race muscles, and the walking footstep's gain is multiplied by the footstep volume.
- The grip relax of the second-hand, arm and extra links is computed in double precision as the original.
- The balance routine's foot sounds (the anchoring foot's rustle, the releasing foot's step and wet step, their clocks) are tested rules, not wired.
- The map records that the grip relax, the weapon flip and the block stamina regeneration were already ported and live.

Phase 3 (Bodies: skeleton, physics, control) in docs/PLAN.md; it also serves Phase 5's audio fidelity (footstep gain).

## Evidence

REA (Ghidra) and `context/re/exe.asm` on the character init, the grip relax and flip part routines, the body tick's block-stamina block and the balance routine's foot sound block; the motiledyn research pack.

## Out of scope

The init's foot-mask union, leg swap, the ghouls' half-scale head-part set-up, the golem contact smoothing, an unread creature word; playing the balance routine's foot sounds; the golem suit's runtime power state.
