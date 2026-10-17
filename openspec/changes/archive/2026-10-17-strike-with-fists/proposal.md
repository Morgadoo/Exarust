## Why

Reference shot 4 of the original shows the unarmed player's punch landing:
the right arm fully out, the body twisted into it. Exarust throws the
"Default" punches, but the fists are no strikers, so a punch never lands;
every click attacks at once, with no cooldown, a dazed fighter attacks as
freely as a fresh one, and holding the button does nothing.

In the original, empty hands are held items with the race's fist or claw
records, and the attack request goes through one executor shared by the
player and the AI: 80 % of the fighter's energy and a cooldown since the
last attack tick (400 ms with no primary striker, else 300 ms × its
recoil + 300 ms: 330 ms for the fists); a held button keeps the request
standing; with the fists only the motion's hand strikes.

Phase 4 (Combat and items), milestone M4 "first duel"; the master plan's
lane C change L3 (milestone MS1, original timing), after L1, L2 and M21.

## What Changes

- Follow-up of M21 now that M40 decodes the race's stats:
  `RaceWeapons` (the race's unarmed reach, swing-turn and recovery factors
  and clawed-hands word) is built from `Race::stats` instead of reading
  the `races.rdb` payload a second time; a character's race weaponry
  (race index and those fields) is kept on its body, and held weapons and
  fists are held by its race.
- Fists strike: an unarmed fighter's dressing gives both hands a striker
  built from its race's fist or claw records (the right by race index, the
  left by the clawed-hands word), a segment from the hand's origin across
  its box (17.66 cm for a human). Model nodes and spawned parts keep their
  box half extents for it.
- The strikers an attack strikes with: with the fists, claws or two
  weapons only the motion's hand (the right in the main side, the stab and
  the overhead, both claws in a clawed overhead, the left in the other
  side); otherwise every held weapon, and the fists not. A fist's contact
  counts at its knuckles, just inside its far end.
- The attack executor (`exa_combat::attack`), shared by the player and the
  duellist: at least 80 % of the most energy; the cooldown since the last
  attack tick on the game clock (12 ms a tick), 400 ms without a primary
  striker (the fists' after equipping), else round(300 × recoil) + 300 ms;
  an overhead unless a stab or an overhead plays, a stab unless a stab
  plays (after the facing gate: the weapon's long axis forward of −0.367
  in the body's frame), a side attack unless a side attack plays. Every
  attack's motion picks the primary striker; equipping a weapon makes it
  primary.
- The player's request, each behaviour tick: a press requests an attack
  (the side taken at the press, an overhead within 300 ms of the press
  before, a stab with Alt and a held weapon), holding keeps it standing
  (attacks repeat after each cooldown; the heading follows the cursor),
  release drops it. A press between two ticks still counts. Crouching
  would force Attack Mid (no crouch in Exarust yet). A double click no
  longer cuts a playing side attack short: the original's free executor
  does not, and its immediate overhead after a released, stalling swing
  comes with the swing clock (M22).
- The duellist's strikes go through the same executor (energy and
  cooldown); its reach ignores its fists.
- Debug helper: `--pause-on-hit <n>` pauses n ticks after the player's
  first blow lands (for screenshots of a landed punch); the log names
  each attack's game time, energy, last attack tick and cooldown, when a
  request waits for energy, and whether a hit came from a fist.
- Docs: PLAYER.md (Combat mode), DEBUGGING.md, formats/OTHER.md (Weapon
  record: fists as strikers), PLAN.md (Phase 4), crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly (`context/re/exe.asm`),
  re-read this change: the executor (held count, the busy branch, the
  cooldown 400 ms or round(300 × W17) + 300 with 300.0 single precision,
  the last attack tick plus cooldown not after now, Break Tangle, the
  double-click, thrust and side branches, side attacks only outside motions
  5–10); the swing routine stamping the game clock as the last attack tick
  on every attack tick; the behaviour tick's press, hold and release, the
  double click, the side test, the heading and aim set to the cursor while
  the request stands, Alt only with hands bits, the 0.8 × max energy gate
  in double precision (also before the AI's two executor calls); the
  starters' primary and strikers, the crouched pick, the wrappers clearing
  the side; the strike window's striker choice by weapon class and motion;
  the stab's facing gate (forward axis (0, 0, 1), round(−15f) < 6); the
  fists' equip clearing the primary and a weapon's equip setting it
  (`context/re/master-combat.md` §1–2, §4; `combat-stances.md` §7, §13;
  `combat-look-plan-final.md` L3).
- Corpus: every race of the installed `races.rdb` builds its weaponry from
  its stats; the human body model's hands give 17.66 cm fists through the
  spawned parts' bounds (tests skipped without `EXANIMA_DIR`).
- Exarust in game through `gpu_run.sh` (light runs): punches landing on the
  duellist's chest with damage, the cooldown spacing, the energy gate, and
  a landed punch compared with reference shot 4.

Out of scope:
- the swing clock, the release and stall, the busy executor's overhead and
  Feint, Remise (a perk), the attack's swing sounds (M22); the aim nudges
  that steer fists (M23); the AI's hold and release (M22, L11);
- detection by contact between bodies, the fist's own damage path (zone 0.2,
  the hand box's ζ, the claws' doubled impact), fists' bites (C21, M30);
- parries (L10); a crouch input; the original's 300°/s combat turn (M2);
- weapon blood never staining fists (L8, to merge: fists have no stain
  material).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: player attacks (request, hold, release), the attack executor,
  fist strikers, weapon contacts by striker.
- `world-viewer`: `--pause-on-hit`.

## Impact

- `exa_formats` (`RaceWeapons::of_stats`, the payload reader removed)
- `exa_assets` (model nodes keep their box half extents), `exa_world`
  (`PartBounds::half_extents`)
- `exa_physics` (`BodyTicks::millis`, the 12 ms game clock; `MAX_ENERGY`
  public): small edits in lane A's file
- `exa_combat` (new `attack` module; `Blade::slot`, `Attacking` motion and
  weapon class, `start_attack` with the primary, contacts by striker, the
  knuckles; `Fighter` requires `Attacker`)
- `exa_character` (`RaceWeaponry`, fist strikers at dressing, held records
  by race)
- `exa_game` (`player`: the request each behaviour tick, the button latch;
  `sim_control`: `--pause-on-hit`), `exa_ai` (the executor gate, reach
  without fists): small edits in lane D's files
- `exarust` (race weaponry on spawned characters, `--pause-on-hit`)
- `docs/PLAYER.md`, `docs/DEBUGGING.md`, `docs/formats/OTHER.md`,
  `docs/PLAN.md`, crate READMEs
