## Context

The user's two reference shots of the original's torch arm
(`context/re/refs/torch-arm-refs.md`) show a compact carry before the
chest. Exarust's exploring player holds the torch up beside its head. The
question was which part of Exarust bends the arm differently. None does:

| suspect | the original (REA, live read-only) | Exarust |
|---|---|---|
| grip | the torch under the right hand at (−12.00, −1.50, 6.00) cm; its turn equals (70°, −15°, 20°) applied X, Z, Y to 0.04° | `MAIN_GRIP`, glam YZX |
| overlay | the motion constructor copies each clip pointer the overlay defines over the parent's table: whole clips, no per-bone mask | `MotionSet::clip` falls back through Torch → One Handed → Default; the same table |
| muscles | Cmbt - Idle: arm stiffness 0.4 both arms, part factor 0.5, relaxation 0.36 (the cap), saturation 0.1; no arm state for a held light | `body_muscle.rs`, the same values |
| weight | torch link mobility 4.31 against the hand's 0.225: the hand takes 5 % of a grip correction | the torch has no physics body |
| tracking | arms within 0.52° (upper arm), 0.81° (forearm), 0.08° (hand) of Cmbt - Idle over 300 samples | 0.41°, 0.48°, 0.05° (character view); Idle within 2.6° in play |

What differs is the clip. The shots are of combat mode: shot 2 has the red
cursor ring (the original's combat cursor, colour ff4c4cb4), and the
user's running game reads combat mode on, motion 4 (Cmbt - Idle) with
stance 3 (Defence) at the exanima01 start, unchanged for 30 minutes and
more. The original's exploration steering sets stance 0x13 (Idle), so
exploring it plays "Torch" Idle, as Exarust does.

The clips' arm, on the body model's bone offsets in the chest's frame:

| clip | elbow | upper arm out / forward | wrist from the shoulder (cm) |
|---|---|---|---|
| Torch / Idle | 122.6° | 39.8° / 11.4° | (−16.4, 1.4, 18.7) |
| Torch / Cmbt - Idle | 101.5° | 24.9° / 31.8° | (−8.6, −3.0, 31.6) |
| Torch / Walk | 125–129° | 37–49° out | |
| One Handed / Defence stance | 82.6° | | the flame 58 cm before the head |

How the original gets from combat mode back to exploring:
- the Tab handler toggles the flag and starts no clip;
- the exploration steering writes stance Idle through the set-stance
  helper every tick, which starts Idle only when the playing clip is the
  old stance's; nothing else in its call graph starts a clip (its three
  indirect calls are line-of-sight queries on the level);
- the body tick's Cmbt - Idle branch (readiness, condition 8, the
  auto-parry check) jumps past the posture switch;
- the locomotion branch starts Walk when the player moves, and the walk
  ends in the stance once the body stands.

So the combat idle that the player keeps in combat mode once its stance
turns back to Defence stays after Tab, until the player walks. Exarust's
exploration steering instead starts Idle on every standing tick.

The research notes, with addresses, are git-ignored:
`context/re/combat-stances.md` §2 and §14, `context/re/held-torch-and-skin.md`,
`context/re/body-q8-q9.md` (arm states), `context/re/master-movement.md` §6
and §8. The format pages this change extends are
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Motion set) and
[`docs/PLAYER.md`](../../../docs/PLAYER.md).

## Goals / Non-Goals

**Goals:**
- The exploration steering starts the clips the original's does, and no
  others: the gaits, ending in Idle.
- The torch arm's two carries pinned to the installed clips, so a later
  change to movesets or clip lookup cannot move them unnoticed.
- The docs say which carry the reference shows and how to see it.

**Non-Goals:**
- Torso twist and head look (M8), the open hand, the walk's loose arms.
- Any change to the grip, overlay, muscles or arm states.

## Decisions

- **Only the gaits.** Moving, the steering starts its gait from any
  looping clip (Cmbt - Idle included, as the original's locomotion does).
  Standing, it ends Walk, Run, Backpedal and the interim Combat Walk in
  Idle; every other looping clip is the stance's. Exarust's stance code
  already has the rest: `set_stance` (the helper) and the posture switch's
  `KEPT` list.
- **Idle, not the stance field, ends a walk in exploration.** Out of
  combat mode the stance is Idle; in the tick Tab turns combat mode off the
  stance rule runs after the steering, so Idle is what the next tick would
  give anyway.
- **The clip test does forward kinematics in the test.** The body's own
  posed matrices come from the solver; the test wants what the clip asks
  for, on the model's offsets (each bone's node translation in its nearest
  bone ancestor), as the research measured it.

## Risks / Trade-offs

- [A balanced player standing in another looping clip out of combat mode
  waits for the posture switch instead of the steering] → the switch runs
  every body tick at balance ≥ 0.8 and tone ≥ 0.85; below that the
  original takes the Defence stance or Rebalance (C11), which Exarust
  lacks either way.
- [The L2 note "Tab → the player idles with its arms down" changes] → it
  held only because Exarust's steering forced Idle; with the stance Defence
  kept in Cmbt - Idle the original keeps the combat idle.

## Open questions

- Does the live original keep Cmbt - Idle after Tab until it walks? The
  user's game stayed in combat mode until it was closed (its last change
  an attack), so this is answered by REA only (call graphs above); a
  read-only watcher of combat mode, motion and stance
  (`context/re/torch-watch/retail_watch2.py`) is kept for the next live
  session (RR: Tab off in Cmbt - Idle, wait, then one step).
- The torso twist rule and the lean on the midline parts (M8, RB20): REA on
  the sampler's twist block, then live samples while the cursor moves.
