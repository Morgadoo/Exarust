## Why

Phase 7 (Thaumaturgy) in docs/PLAN.md, the thaumunits gap lane (round
166): SG_Thaum_Force's unit routines, TForceDomeEffect, TForceBarricadeEffect,
SG_Thaum_Mind and TThaumTree. The ratings left open the cast-hook gate of
the player's behaviour tick (round 124 found it), the dome contact's knock
count, energy drain and route clear (computed, not applied), its struck
call, and the tree's part of the GUI's forms sweep. A GPU run of Control
Mind also showed the handover panicking behind the simulation bridge.

## Evidence

REA (Ghidra and the objdump listing of `Exanima.exe` 0.9.5):

- The player's behaviour tick returns before its cast-hook asks while the
  body's shock-stagger state flag (0x100) is set or the behaviour's own
  disable byte is set; the controls' block, second block and end set that
  byte, their releases and the controls' start clear it. Its walk stops on
  the collapsing condition bit.
- The dome's contact (a class slot the bubble's keep-out calls once for
  each pushed part; the barrier runs the same code inline in its part
  loop) measures from the character's chest point. A held item's part
  (flag 1) only breaks its holder's attack (the struck call) when it is a
  weapon (0x10000) and plays the hit sound. A character's limb within the
  radius plus 20 times the size: the 40 cm shove, the sound, the
  instability counter plus one, the energy above 0.25 lowered by 0.002
  (dome) or 0.001 (barrier) within 100 cm, more beyond (difference in
  single precision, the rest in double), and the AI behaviour's route
  byte cleared.
- The barricade's shove: the push's arithmetic in double, a tenth in
  double behind the wall (and for a NaN depth), the energy loss in double,
  the sound draws in double, and each knock clears the route byte too.
- The tree's sweep entry (two small routines): while its form exists it
  closes it, its bar and its flag; else, when the mask has the bar's bit
  and the bar is toggled open, the bar falls. Control Mind's handover and
  hand-back run the GUI's forms sweep with mask 1 first.

## What Changes

- The player's hook asks stop while shock-staggered or while the
  behaviour is suspended (the controls blocked).
- The bubble and barrier contacts run once for each pushed part, from the
  chest point, and apply the knock, energy drain and route clear; a held
  weapon's struck part breaks its holder's attack.
- The barricade's shove takes the original's double precision, NaN branch
  and route clear.
- The player's bar casts take the body's standing point (its transform's
  place), as the duellists' do: the bubble stands on the ground plus 100,
  not a metre above that.
- Control Mind's handover and hand-back sweep the power tree and bar, and
  the handover, the hand-back's transition and the spirit's end run on the
  frame's world behind the bridge (they panicked on the missing control
  state).

## Impact

`crates/exa_game/src/thaum/{cast,casting,barrier_live,bubble_live,barricade,live}.rs`,
`crates/exa_game/src/effects/force_dome.rs`, `crates/exa_game/src/power_screen.rs`,
`crates/exa_game/src/mind/runtime.rs`. Lockstep traces (exanima01, arena
duel) identical to main.
