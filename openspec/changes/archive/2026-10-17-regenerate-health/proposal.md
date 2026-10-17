## Why

In Exarust a wound never heals: yellow health only goes down, so the
HUD's brown segment never refills and, out of combat mode, a blow shows no
bars. The original heals every character each body tick: yellow creeps
back towards red at a rate that ramps up over 40 s to √red × 0.01 a
second, held at 0 while the character attacks or stands in Cmbt - Idle,
and every blow starts the ramp again; a retail trace fits the rule to
0.001 over 46 s. Red heals only for the races whose rule bits allow it,
and from a heal pool that bandages fill.

Phase 4 (Combat and items), milestone M4 "first duel", and the master
plan's MS5. This is change M43 of the master plan
(`context/re/master-plan.md`, lane F, F6; body note B5), now that the
condition word (M38) and the rule bits (M41) are on main.

## What Changes

- `Health` keeps the regeneration rate and a heal pool beside yellow, red
  and the maximum; `Health::take` sets the rate to 0 (every damage call),
  and so does every weapon contact on a fighter in the hit reaction.
- New `exa_combat::regen`: the health tick (`HealthTick`, after the
  posture stage, before the clips advance), once a fixed tick for every
  fighter: red regeneration (1.25e-5 a tick up to the maximum, dead or
  alive) for rule bits holding 0x20 without 0x100; then, unless dead, the
  heal pool's transfer to red (1.25e-5 a tick), the yellow regeneration
  (the rate 0 while attacking or while Cmbt - Idle plays, else up by
  0.0003 × 0.01/83 a tick to 0.01/83; yellow += √red × rate above a red of
  1.19209e-7) and the clamps (yellow within red, then red within the
  maximum, yellow at least 0).
- `exa_formats::character_stats::BodyRules::regenerates_red`: 0x20 without
  0x100 (the golem suit's race word gives 0x20, but without its core the
  armour pass adds 0x100).
- The collapse's death sets yellow to 0, as the original's death does
  (yellow regenerates during the collapse).
- The HUD needs no change: its bars already show in every frame whose
  yellow health changes, so out of combat mode they now show while yellow
  heals after a blow, as the original's do.

Evidence:
- `exe.asm` and constants read from `Exanima.exe` 0.9.5: the body tick's
  health bookkeeping (heal pool, the gate on condition 2 and motion 4, the
  ramp 0.0003 in double, the cap 0.01/83 as a float in `.data` with no
  writer, √red × rate in float above 1.19209e-7, the clamps' order), its
  prologue's red regeneration (rule bits & 0x120 = 0x20, 0.25 × 5e-5,
  before the dead test), the damage call's and the damage routine's
  resets of the rate, death's yellow 0 (`context/re/master-body.md` §4.2–4.5
  and the new §22).
- The retail trace `refs/combat-live/watch2-trace.log`: yellow 0.085 held
  in Cmbt - Idle, then 0.086, 0.090, 0.100, 0.110, 0.121 at 5.0, 9.6, 16.2,
  21.1 and 25.3 s of Idle, 0.202 at 45.8 s, with red 0.203.
- The installed corpus: which placed characters and arena actors
  regenerate red (Sir and the three harvesters in the story levels;
  the demon in the arena).

Out of scope:
- the second wind, bandages and potions (M46: nothing fills the heal pool
  yet), burning's rate reset (M45), loading the saved heal pool (M47);
- the damage rules themselves (M29: the flag-8 rule, `vital`, the story
  rule), block stamina (M26);
- the HUD's interpolated yellow between ticks (Exarust draws the tick's).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: health regeneration and red regeneration (new
  requirements).
- `user-interface`: the health bars show while yellow heals out of combat
  mode (modified Health bar fade).

## Impact

- `exa_combat` (`regen.rs` with `regen_test.rs`, `fighter.rs`, `lib.rs`,
  `collapse.rs`)
- `exa_formats` (`character_stats.rs`: `BodyRules::regenerates_red`)
- `docs/formats/OTHER.md`, `docs/UI.md`, `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_combat/README.md`
