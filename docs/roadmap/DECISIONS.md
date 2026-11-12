# Decisions

Decisions that shape the [roadmap](README.md). An open decision carries a
recommendation and what it blocks. When the owner answers, move it to
"Taken" with the date.

## Open

### DEC-1. What 100% means
**Question:** is the project complete when every function is in a final
ledger state (live, engine-replaced, unreachable, not-applicable, approved
deviation), and the scenario matrix passes against retail?

**Recommendation:** yes. Without closing states for engine-replaced and
unreachable code, no amount of porting reaches 100%: the count includes GL
plumbing, threads and dead code.

**Blocks:** A6 (the published number), B2, B3.

### DEC-2. Steam
**Question:** link the Steamworks library, or close the Steam code as a
kept deviation?

**Facts:** Steam API glue is rated 7%, Steam Input 75% (the action sets
are reproduced on gilrs; Steam is not linked). The retail game ships
`steam_api64.dll`. Linking it ties the build to Steam's licence terms and
to a running Steam client.

**Recommendation:** keep it as a deviation for now: gamepads work without
Steam, and Steam adds achievements and the overlay only. Revisit at
packaging (task D5 of Phase D).

**Blocks:** part of B4 (platform), D3 of Phase D.

### DEC-3. Character creator
**Question:** do SG_CharacterGen, SG_Character_Create and TCharCreation
stay with the owner's other session?

**Facts:** about 9 KB of the estimated gap. This session does not touch
those files.

**Recommendation:** audit them in Phase A like everything else (read
only), and leave the ports with the other session.

**Blocks:** B6.

**Note (2026-11-12):** the slot-kind fix (round 203) left three creator-side items for the other session: `create.rs` `SaveKind` names are swapped (numbers correct, not wired); the `continue_label` comment; the character-creation spec's continue-button wording.

### DEC-4. Start and budget of Phase A
**Question:** start the audits, and at what scale?

**Facts:** 90 audit lanes and 90 reviews; an estimated 30 to 45 million
tokens and two to three working days at eight lanes at a time. The three
pilot packages (A3) cost about 1 million tokens and correct this estimate.

**Recommendation:** approve the pilot now; decide the rest on its numbers.

**Blocks:** A3 onward.

### DEC-5. Kept deviations
**Question:** confirm each difference kept on purpose.

| item | why it was kept |
|---|---|
| Supersample jitter pattern | a rotated grid, kept by an earlier decision for picture quality |
| Path planner's shared stamp table | reproducing it would break determinism |
| Acceleration trees (kd-trees, block trees) | Rapier colliders give the same answers |
| Screen-saver refusal, work-area fit | no meaning on Linux |
| Quad-buffer stereo | not reachable on current hardware |

**Recommendation:** approve all five; each becomes a `deviation` row with
this entry as its reference.

**Blocks:** B5.

### DEC-6. Composite terrain draw
**Question:** make the original's composite terrain draw the default?

**Facts:** it is ported and opt-in. No retail outdoor reference exists to
confirm the picture.

**Recommendation:** decide after C2 captures outdoor references.

**Blocks:** part of B4 (world), two terrain components' last gaps.

## Taken

| date | decision |
|---|---|
| 2026-11-12 | DEC-1 to DEC-4 taken as recommended when the owner set the goal "complete the plan": the ledger definition of 100%; Steam kept as a deviation for now; the character creator audited read-only, ports left to the other session; Phase A starts with the three-package pilot, the rest sized from its numbers. |
| 2026-11-11 | Lockstep traces may change when a change makes timing follow the simulation as the original does (round 201); each change is explained in the round's notes. |
| 2026-11-11 | The bar's casts draw from the simulation's random streams (round 197), as the original shares them; one sweep scene's outcome changed. To be confirmed by B0.4. |
| 2026-11-11 | Project paused after round 202 at the owner's request. |
