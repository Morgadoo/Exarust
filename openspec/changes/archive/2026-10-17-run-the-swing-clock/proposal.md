## Why

An attack in Exarust is a clip that always plays to its end at its
weapon's rate: a click and a held button swing alike, a struck attacker
snaps back to its stance at once (an interim stand-in), a released swing
cannot be followed by the original's quick overhead or Feint, Remise never
chains the other side, and the strike window is Exarust's own 35–85 % of
the clip.

In the original, the attack branch of every body tick runs a swing
routine: the clip clock's rate from the weapon, a swing strength P that a
stall drains (a released swing, a struck one) until the stance returns,
the strike window per motion, and Remise's chain past 80 % of a held side
attack. The behaviour releases a swing whose request was dropped, and the
executor lets a released swing's request throw the overhead or, with
Feint, renew the attack. The player's live trace shows the held two-hander
chaining its combo 828 ms after each Attack Mid starts.

Phase 4 (Combat and items), milestone M4 "first duel"; the master plan's
lane C change M22 (milestone MS1, original timing), after L3 and C10.

## What Changes

- The swing routine (`exa_combat::swing`), every attack tick of a fighter
  whose condition is attacking, before its clip clock advances: the last
  attack tick stamped; the striker trackers in the window region; the
  rate (swing speed × g × 1.125 × base, the stab's shaping, and now the
  feral level's ramp); a chained swing's carried window closing past 25 %;
  the overhead's and stab's windows closing late; the window opening past
  30 % (stab 40 %, overhead 50 %), with the fists, claws or two weapons
  only the motion's hand, which becomes the primary striker; Remise past
  80 %; the stall draining P (the clip × P) and, once its end has passed,
  the stance (blend 2·W19); tiredness ((E/Emax)², none under 0.8).
- The swing state on every fighter (`SwingState`): the original's state
  word, P, the stall rate, the stall's end, Feint's renewal time, the rate
  set this tick, the strikers' window and trackers. The body tick reads
  the rate for an attack's clip.
- Release: a dropped request during an attack (player) stalls the swing
  from P = 1 by max(1 − 0.05·|v|, 0.1)/W17 × 0.03 a tick (at least 0.01),
  the stance back after 1000/(80 × stall) ms, at most the expected rest of
  the swing (the original's estimate at 80 ticks a second); Feint's
  renewal after (2f + 1) × (round(100·W17) + 100) ms. A released main-side
  attack returns to the stance once the stall is over (2·W19, 30 frames
  without a primary).
- The executor's busy path: while a swing stalls only a released swing's
  request starts anything: the overhead (no blend) within 300 ms of the
  side attack's start, else with Feint a stab or an overhead (blend 10) or
  a side attack. Free, a released swing's request still gives that
  overhead, and Feint waives the cooldown from its renewal time.
- Remise and Feint are the fighter's record perks (`Fighter::perks`):
  placed characters carry their records' perks; the player and the
  duellist are new records (none). A Remise chain starts the other side's
  attack with round(6·W18) + 2 frames of blend, its window carried.
- The struck attacker's stall replaces L2's interim "stops its attack and
  returns to its stance": the daze D (at most 0.9) stalls the swing by
  4·D·f − (1 − D)·min(0.005·s, 1)·W17 a tick when that is at least 0.001
  (f = 0.25 over the most health, halved under a potion), the stance back
  after 80·P/stall ms, at most the swing's expected rest.
- Hits come from strikers whose window is open (the held items; the fists
  only when they are what is held), replacing the 0.35–0.85 window; the
  damage reads the original's swing clock.
- While attacking the player's body turns at (1 − 0.6·enc)(1 − 0.25·p)·
  B·W16·P/scale a tick (W16, the record's handling, now has its reader).
- The attack picks (random without repeat) move to the fighter's
  executor state, so Remise picks the chained clip.
- Debug helpers: `--perks a,b,...` gives the player perks; `--trace` gains
  the swing columns (rate, strength, state word, as the retail probe's);
  logs for releases, stall ends, chains and struck swings.
- Docs: PLAYER.md (Combat mode), formats/RSQ.md (Motion playback, the
  trace columns), DEBUGGING.md, PLAN.md, crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and `context/re/exe.asm`, re-read for this
  change: the swing routine whole (order, region, tracker, rate with the
  feral term and the dead rate-zero branch, carried window, window
  closes and opens, Remise, stall, energy override in double precision),
  the release, the daze and its stall, the expected swing time, the
  smoothstep, the executor's busy and free paths, the four wrappers (side
  attacks ignore the executor's blend; overhead and stab take it), the
  side starters' chain and fresh paths, the overhead starter, the clip
  start (it closes every striker's window), the player's no-request
  branch, the body turn's attack law; constants read from the PE
  (`context/re/master-combat.md` §1, §3, §4, §7, and its "M22 findings").
- The retail game was not run: the live trace's 828 ms Remise chain
  (`refs/combat-live/watch2-trace.log`) is the reference.
- Exarust in game through `gpu_run.sh` (light runs): held punches 67
  attack ticks, clicks checked in 5, Remise chains at 55 ticks with the
  fists and 78 with a greatsword, struck swings stalled.

Out of scope:
- the attack arm stiffness (W20·L(p), per-arm saturation): it needs
  per-arm memories in the body controller (lane A, after M0);
- the swing whoosh and attack voice (no sound sets yet; M0 splits
  `exa_audio`);
- the weapon-in-hand gate (weapons are not bodies until M24), the
  ×0.64 rate under a flag no writer sets (RB9), the striker speed flag
  set over 50 cm² a tick² (RB14);
- the AI's hold and release and its attack choice (L11): the duellist
  holds every swing;
- the follow-through and cleave after a wound, recoil (C21, M26); the
  aim nudges (M23); parries (L10); the counter-guard (M27).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: the swing routine, release, struck swings, Remise, Feint, the
  busy executor, the strike window, the attack turn.
- `world-viewer`: `--perks`.
- `physical-body`: the body trace's swing columns.

## Impact

- `exa_combat` (`swing`: `SwingState`, the routine, release, daze stall,
  expected time, turn rate; `attack`: `Busy`, `Start`, the busy path,
  `Side`, the picks on `Attacker`, `is_striker`; `start_attack` takes the
  swing state and an `AttackStart`; detection by open windows; the struck
  attacker's stall; `Fighter::perks`)
- `exa_physics` (`MAX_DAZE` public): a one-word edit in lane A's file
- `exa_game` (`limp`: the routine's rate for an attack's clip; `player`:
  release, the stance return, the executor's state, the attack turn;
  `trace`: swing columns): small edits in lanes A/B/D files
- `exa_ai` (the duellist through the new executor and start): lane D
- `exarust` (`--perks`, placed characters' perks)
- `docs/PLAYER.md`, `docs/formats/RSQ.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, crate READMEs
