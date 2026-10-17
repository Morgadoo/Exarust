## Context

The original applies every blow's health damage through one call, made by
two routines: the weapon damage routine (vital always) and the impact
routine (vital once its accumulated impact passes 0.6). The call resets the
regeneration rate, takes yellow and red by the victim's rule bits and
state flags, and decides death; a blow that leaves yellow low without
killing runs a low-health check (second wind, else the player's
heartbeat). Three other places read "yellow under 0.01" on a living
character. The research notes, with addresses, are git-ignored:
`context/re/master-body.md` §4.2 (the rule), §4.7 (second wind), §4.10
(low yellow), §7.4 (rule bit 0x8), `hud.md` §6 (the heartbeat) and the new
§23 (this change's reading, RB48).

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Damage:
Applying a blow); the heartbeat is in [`docs/UI.md`](../../../docs/UI.md).

## Goals / Non-Goals

**Goals:**
- the damage call's order, branches and precision on `Health`, a pure
  method pinned to worked numbers (the ratio after the yellow cut, the
  feral rule, the potion, the double-precision death line, non-vital
  damage, the story's red loss);
- the low-health check as an event the HUD's heartbeat (and later the
  second wind) reads;
- the player's stance reading the same "under 0.01" test as the original.

**Non-Goals:**
- second wind, potions' timer, bandages (M46); impacts (C22); the block
  path (M26); burning (M45); the stance by energy and parries (M22, M26,
  L10/M27, lane D); the daze's factors (M30).

## Decisions

- **One method, the original's arguments.** `Health::take(Damage,
  BodyRules, StateFlags, story) -> Taken`. `Damage { yellow, red, vital }`
  carries the call's three numbers (`Damage::blow` makes a weapon blow's,
  always vital); the rule bits and state flags are the victim's own
  components; `story` is the game's mode. `Taken` says which way the call
  ended: `Hurt` (yellow still 0.05 or more: nothing else runs), `Weakened`
  (the low-health check runs), `Killed` (the collapse starts). The caller
  acts on it, as the original's call does inside.
- **Precision.** As the original: each subtraction in double precision on
  the float values (0.1, 0.05 and 0.2 are doubles), the red ratio in float
  (red_new / red_old, then times yellow), the 0.05 test in float, the 0.01
  test in double. So a yellow of exactly 0.01 in single precision
  (0.0099999998) dies, where Exarust's float test kept it alive. The old
  rule's extra "red was above 0" test goes (the original has none).
- **Red is not clamped.** A heavy blow can leave red below 0; the call
  sets yellow to 0 then. The regeneration's clamps keep yellow at 0 after
  (yellow within red, then at least 0), as the original.
- **The "no death" byte and the player hook.** The call also spares a
  character whose byte at one offset is set, and calls a player hook on a
  killing blow. Neither is ever written in the executable (no store, no
  address taken, no property table entry; the hook's slot is only read),
  so both are left out.
- **Story mode.** The story byte is written only by the game-mode setup:
  1 for the story, 0 for practice and the arena, together with the flag
  M41 reads for rule bit 0x10. Exarust has the same split in
  `exa_world::start::mode`; the viewer inserts `StoryMode` for story
  levels. Without the resource (tests, other views) it is not the story.
- **The low-health check as a message.** `Weakened { fighter }`, written
  by the hit reaction when `take` returns `Weakened`. The HUD's heartbeat
  reads it for the player's body (keeping its once-a-minute and
  story-only rules) instead of inferring it from the HUD's yellow, which
  missed a non-vital blow to 0 and a blow dealing nothing to a body
  already under 0.05. M46 adds the second wind in front of it; M26's block
  path writes the same message.
- **Low yellow.** `failing(yellow)` is the double test. The player's
  stance uses it. The stance by energy and the auto-parry's clearing also
  test it, but their callers are not ported; they read `failing` when
  they land. Today only `--health` makes a living fighter fail (the
  weapon blow is always vital).
- **Debug flag.** `--feral f` sets the player's record's feral level, so
  a duel shows the no-lasting-wounds rule (red stays full, yellow takes
  both shares) and the feral stats (energy cap, head, walking arms).

## Open questions (answered)

- *Who writes the "no death" byte (RB48)?* Nobody: two readers (the
  damage call, the get-up decision), no writer by displacement, address
  or property table; the object is zero-filled when made.
- *Is the weapon routine's vital argument ever false?* No: it comes from
  a function that returns 1.
- *Which mode is "story"?* The setup's mode 4, the one that installs the
  story prelude; practice (2) and the arena (8) clear the byte.
- *Does the low-health check run when yellow stays at 0.05 or more?* No:
  the call returns at once.

## Risks / Trade-offs

- The hit reaction writes `Weakened` after the update, the HUD reads it
  in the next frame's update (messages live two frames): the frame in
  which the HUD used to see the yellow fall, so the heartbeat starts no
  later than before.
- The double-precision subtraction changes Exarust's yellow and red after
  a blow by a few units in the last place; the duel's outcome does not
  depend on it.
