## Context

`shade-and-paint-blood` painted wounds only from `--debug-wound`. In the
original, the damage routine's blood branch queues a wound on the victim
when its race bleeds and R > 0.05; the victim's body tick paints it seven
ticks later with the painter that change reproduced.

Exarust detects hits once a frame in `PostUpdate` (after the frame's body
ticks and transform propagation): a blade (the entity holding the weapon
model, local +Y) swept through a body part's sphere. The body solver runs
at 83 Hz in the fixed step and counts its ticks (`BodyTicks`).

This change extends [`docs/RENDERING.md`](../../../docs/RENDERING.md)
(Blood) and [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Damage). Notes with addresses are git-ignored
(`context/re/wounds-and-blood.md`, `combat-look-plan-final.md` L7).

## Goals / Non-Goals

**Goals:** blows bleed exactly when the original's do; the queue's size,
delay and one-a-tick order; the smear from the striker's follow-through;
corpses keep painting; fists never bloody themselves.

**Non-Goals:** sprays, blade stains, creature self-blood, collapse, fists
that strike, blood across re-dressing or saves.

## Decisions

- **R beside red.** The damage model already computes the lasting damage
  on the part; `Outcome::wound` keeps it before the red bar's 0.75, so the
  test is on the original's quantity instead of dividing `red` back.
- **Bleeding on the fighter.** `Fighter::bleeds` defaults to true (humans:
  the player and the duellist); placed characters take it from their race
  record (`Fighter::of_record`, `Race::bleeds`).
- **The queue in the combat, the painting in the character.** The queue
  is combat state (`WoundQueue`, required by `Fighter`, so every fighter
  has one); `exa_combat` counts it down and sends `Bleed`; the blood
  module turns each into the `PaintWound` it already paints. The striker's
  current point is read in the combat, which knows the striker.
- **Counting body ticks in `PostUpdate`.** The countdown runs first in the
  combat's frame systems: it counts each queue down once for every body
  tick run since the last frame (one paint a tick at most), then the hits
  of this frame queue their wounds. A wound struck this frame therefore
  waits from the next tick on, and the travel is read on the pose this
  frame draws, as the victim's vertices are.
- **Weights in the striker's tetrahedron.** The original anchors the
  contact point to the striking body's four particles by barycentric
  weights. Exarust's blades move rigidly with their entity, so their
  tetrahedron is the entity's origin and the ends of its three axes; the
  weights are then the point's affine coordinates in the blade's frame,
  and stay valid for a body with four particles (C21's weapon bodies).
  A striker gone or without a frame gives no travel (a ball).
- **Fists.** The queue only ever paints the victim. The original's
  striking-hand slot keeps punches from bloodying the puncher; Exarust has
  no self-painting, so a fist stays clean by construction (tested with a
  hand as striker). Creatures' self-painting slot is not done.
- **Memory.** No new copies: wounds from hits copy a struck character's
  meshes on their first change, as test wounds do (`shade-and-paint-blood`).
  In the arena a body in braies copies 6–7 meshes (98–111 KiB) on its
  first wound; in the duellist's outfit 8 (241 KiB) on the first and 66
  (1.1 MiB) after three.
- **Debug clicks.** `--click` presses the left mouse button at frames, as
  `--press` does keys, so an armed player in combat mode attacks in
  unattended runs. The hit log names the struck body and the tick.

## Open reverse-engineering questions

- Whether the original's hit tick counts down its own wound (6 ticks after
  the blow) or the next one starts (7). *Answered (REA, in review):* the
  damage routine runs in the striking character's post-step, after the
  tick's integration and contacts, while every body tick (the countdown)
  runs at the start of a tick. The first count is the next tick's, so a
  wound is painted on the 7th tick after the blow, as Exarust does. The
  striker's point is then read before that tick's integration, so the
  original's travel covers six ticks of follow-through; Exarust reads it on
  the frame drawn after the seventh tick, within its once-a-frame hit
  detection's tick of uncertainty.
- The striking body's four particles for a held weapon. *Answered (REA):*
  the weights are taken against the striking body's particle array; for a
  rigid blade any four non-coplanar points of its frame give the same
  point, so Exarust's frame tetrahedron is equivalent.
- Re-dressing and saves. *Open, retail requests R2/R3:* Exarust never
  re-dresses a character yet.
