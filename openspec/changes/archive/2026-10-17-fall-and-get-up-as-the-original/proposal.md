## Why

Exarust's falls and get-ups are still its own: any body under a balance of
0.6 falls into Unconcious 1, and gets up only after lying 2.5 s with its
energy back to 0.9 and its centre of mass still, with the get-up clip whose
first pose looks most like the body. The original decides all of it in the
body tick's posture block from last tick's balance: upright bodies steady
themselves with the Defence stance or Rebalance, moderate leans stagger
into a get-up played partway, far, sideways, fast or lost leans fall, a
body lying still gets up as soon as its feet give support (or 100 ticks
after they lost it) with the get-up its pelvis says, turned the way it
rises, and an unsteady standing body steadies itself. The posture block is
also where the condition, the collapse's dying tick and the fall check
run; with the falls ported they can join each character's own posture
stage.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man"
(stumbles when pushed, falls and gets up), the master plan's MS2. This is
change C11 of the master plan (`context/re/master-plan.md`, lane A, A4),
on main's C6–C10, M0's stages, M38's condition and L5's collapse.

## What Changes

- New posture rules in `exa_game::balance` (replacing Exarust's
  `fall_off_balance` and `get_up`): `respond` (Defence, Rebalance, stagger,
  fall, by the lean, the frame's forward axis, the slow chest speed, the
  balance and the offset, in the original's order and double compares),
  `steady` (the Defence stance or Rebalance under 0.8), `pick_rise` (±0.2
  hysteresis), `waits_for_support` (100 ticks), `rise_heading` and
  `turns_to_rise` (cosine under 0.95), `fall_heading`, `rising_energy`;
  systems in the character tick: the posture actions after the condition
  and the dying tick (`take_posture`: starts eased fast, a Rebalance's
  wait, the fall off the controller, the stagger's jump in, the get-up
  start, the weak and unsteady stances, the off-feet mark), the get-up
  picked again in the movement stage while under 40 % (`pick_again`), the
  turn toward a fall (`turn_toward_fall`), a get-up's extra energy in the
  readiness stage (`recover_rising`).
- `exa_anim`: `Animator::capture_key` and `Animator::jump_in` (a start a
  few keys in; the sampler drops a capture meant for another key).
- `exa_physics`: `Fallen` becomes a marker of a body off its feet;
  `Drive::posture_wait` (the Rebalance's hold on the posture switch);
  `Ragdoll::turned` (a heading turn the next aim does not carry the body
  through); `Balance::fast_velocity`.
- `exa_combat`: `track_condition` and `dying_tick` run in each character's
  posture stage (`CharacterStage::Posture`), the health bookkeeping in its
  upkeep (`regenerate_red`) and attack (`keep_health`) stages, the global
  `regenerate_health` only for fighters the character tick does not tick;
  `ticked_lives`, the run condition by which a body that dies in its
  posture stage skips the rest of its tick; the daze's stance start is
  gated by the condition alone (the original's quirk: a dazed body still
  falling restarts its fall every tick); the posture switch waits out a
  Rebalance.
- `exa_game`: the fall check in the posture stage; the bearing reads the
  posture branch and the condition (off balance, getting up, rising); the
  get-up clock reads the armour encumbrance; later stages run only while
  the ticked body lives.

Evidence:
- `exe.asm` and the PE (objdump at the routines' starts, constants read
  from `.rdata`/`.data`): the posture block, the get-up start, the
  feet-flag clearer it calls, the clip start's no-op test, the daze block,
  the standing branch's unsteady stances and posture switch, the
  posture-switch countdown and its decrement, the heading toward a fall
  and its call site, the re-pick's call site, the energy recovery while
  getting up, the collapse branch's exit on death. RB1 (Q11) answered.
  Findings: `context/re/body-feet.md` §16.
- The installed corpus: the posture clips' action indices, modes and the
  get-ups' key times (`humanoid.rsq`).
- Light runs on the RTX 4000 (`--fixed-frames --shove 30/60/90`,
  `--opponent`, the regeneration run).

Out of scope:
- recovery steps (C15), the feet's contact states the get-up clears (C12),
  the pins it lowers while getting up (C14), the probe height of a falling
  body not yet down (the feet routine, C12);
- the knock-out, dormancy and shocks that set the state flags the posture
  block tests (C23, outside); the no-death byte's facing-only get-up (no
  writer, RB48); the look and aim directions the fall's heading turn also
  sets (M8, M51); the held items' pair group (M24);
- the leg layer restored on leaving a state of 0x100 or more (nothing in
  Exarust that lowers it comes back);
- RR7, the retail knockdown capture.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: falls and get-ups, the character tick's stages
  (modified); staggers, getting up from the floor, rising energy,
  steadying on the feet, turning toward a fall (new).
- `character-stats`: the posture decision runs in each character's
  posture stage (modified).
- `motion-playback`: the get-up clock reads the encumbrance (modified);
  starting a few keys in (new).
- `combat`: the dazed stance's gate, the posture switch's wait (modified).

## Impact

- `exa_game` (`balance.rs` with `balance_test.rs`, `fall.rs`,
  `fall_test.rs`, `limp.rs`, `stages/mod.rs`, `stages/posture.rs`,
  `stages/sampler.rs`, `trace.rs`'s test)
- `exa_combat` (`condition.rs`, `collapse.rs`, `regen.rs`, `stance.rs` and
  their tests; `lib.rs` test) — lane F's and C's files, small edits
- `exa_physics` (`body.rs`, `body_balance.rs`)
- `exa_anim` (`clock.rs`, `lib.rs`, `clock_test.rs`) — lane B's file, small
- `docs/formats/RSQ.md`, `docs/formats/OTHER.md`, `docs/PLAYER.md`,
  `docs/PLAN.md`, the READMEs of `exa_game`, `exa_combat`, `exa_physics`,
  `exa_anim`
