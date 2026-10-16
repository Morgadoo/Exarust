## Why

The user's screenshots of the original's arena show an unarmed fighter
throwing a straight punch, the right arm out and the body twisted into it
(shot 4), and the fists-up guard (shots 1 and 2). Exarust's player uses
the "One Handed" moveset in combat mode even with empty hands, so an
unarmed attack is a sword swing with an empty hand, and the "Default"
moveset's punches and guard are never reached.

The original picks a character's moveset from what it holds, after every
equip change and in or out of combat: empty hands fight with the race's
own moveset ("Default" for a human), whose attacks are the punches. A
left click picks the attack: the right straight, the left straight when
the cursor lies more than 20° left of the facing, or an overhead on a
double click.

Phase 4 (Combat and items), milestone M4 "first duel": the first of the
combat-look changes (`context/re/combat-look-plan-final.md`, L1).

## What Changes

- What a character holds picks its moveset, stance class and weapon
  class, as the original's rule: empty hands the race's own moveset
  (class 5, weapon class 0x80, the fists); a one-handed weapon "One
  Handed" (1); two of them "2W Light" (6, 0x90); a two-hander "Two
  Handed", "2H Light" when its kind bit 0 is set, or "Greatsword" for the
  greatsword's flags 0x521 (2); a polearm "Polearm" (2); a shield "1H &
  Shield", "Shield C1" or "Shield C2" (3); a light in the off hand "1H &
  Torch" (4); a light in the main hand "One Handed" with the "Torch"
  overlay (5). The player and the duellist follow it in and out of combat
  mode; a clip playing goes on in the new moveset.
- Dressing puts a wielded weapon's flags in the main hand and lighting the
  carried torch puts it in its hand, so the rule sees both.
- The player's left click in combat mode picks the attack as the
  original's: Attack Mid, Down or Up at random, never the same twice in a
  row; Attack Combo 1–3 the same way when the cursor lies more than 20°
  left of the body's facing (15° for weapon class 0); Overhead Swing on a
  click within 300 ms of the one before. Unarmed, these are the
  "Default" punches: the right straight, the left straight and the right
  overhand. They cannot land yet: the fists are not strikers until L3.
- Combat facing, which the side choice needs: the heading turns to the
  cursor once it is more than 30° off, and follows it while the left
  button is held; the body turns toward the heading at the body-turn rate
  (240°/s, eased over the last 0.6 rad) instead of snapping to the cursor.
- The player's get-up ends in its combat or exploration stance by its
  combat mode, no longer by its moveset's name.
- Debug helpers: `--press` takes the mouse buttons (`LMB`, `RMB`, `MMB`),
  and `--cursor x,y` fixes the cursor at a window point in play mode.
- Docs: PLAYER.md (moveset from held items, combat facing and attacks),
  OTHER.md (weapon flags), DEBUGGING.md, PLAN.md (Phase 4), crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly: the moveset routine run
  after every equip change (held items, shield, off-hand light, the hands
  bits, 0x521, kind bit 0, the fists' 0x80/0xC0); the motion constructor's
  moveset names per configuration; the fists' built-in weapon records
  (flags 0, or 0x40 for the two clawed ones); the behaviour tick's attack
  request (side by the wrapped cursor yaw minus the body yaw beyond 15° or
  20°, the 300 ms double click), its heading rule (the cursor direction
  once more than 30° off, and at every attack request) and the attack
  executor (an overhead not over a stab or overhead); the two pickers (the
  global 32-bit generator, redrawn while equal to the side's last pick)
  (`context/re/combat-stances.md` §1, §7; `combat-look-plan-final.md`).
- Corpus: the movesets of `humanoid.rsq` and their parents
  (`exa-tool rsq`); every weapon item of `objdb.rdb` mapped to a moveset
  that the installed motion set resolves (the new test).
- Live retail (read-only probe, earlier session): a two-handed sword gave
  stance class 2, weapon class 0x20, one held item.
- Exarust in game through `gpu_run.sh`: the character view's Default
  attack clips and the unarmed duel with scripted Tab and clicks.

Out of scope:
- the combat stance (Defence stance or Cmbt - Idle by the target's
  distance) and the guard at fight start (L2, `take-the-combat-stance`);
- fists as strikers, the attack cooldowns (400 ms, 300·W[17] + 300 ms),
  the energy gate (E ≥ 0.8·Emax) and holding the button to repeat or
  chain attacks (L3, C21);
- the daze's stance and pose hold (C10), parries (L10);
- shields, a second weapon and their equipping (no Exarust item holds
  them yet), the "Torch" overlay over a shield moveset, and the armed
  movesets of non-human races (looked up under the race's own moveset);
- crouching (an attack from a crouch is always Attack Mid), Alt thrusts,
  the shield and dual counters to an opponent's parry, the AI's own
  attack choice (L11).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: the moveset from held items; the player's attack choice.
- `player-movement`: combat facing turns toward the cursor beyond 30°.
- `world-viewer`: scripted mouse buttons and a scripted cursor.

## Impact

- `exa_character` (new `held` module: `Hands`, `HeldMoveset`, `choose`,
  the system switching clips; `equip_outfits` fills the main hand)
- `exa_game` (`player`: facing, attack choice, `ScriptedCursor`; `torch`:
  the lit torch fills its hand; `balance`: the player's get-up stance)
- `exarust` (`--press` mouse buttons, `--cursor`; the duellist's hands)
- `docs/PLAYER.md`, `docs/formats/OTHER.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, `crates/exa_character/README.md`,
  `crates/exa_game/README.md`
