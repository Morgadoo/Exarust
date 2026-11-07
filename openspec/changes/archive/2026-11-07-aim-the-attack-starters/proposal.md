## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart
gap lane (round 125). The routine table built against the current code left
the four attack starters partial: Exarust drew the side attack's clip at
random for the player, always played the side's first clip for the AI, never
aimed a pick at the foe's parry, never wrote the countered motion, and the
two-weapon immediate counter was assumed to start without starting. The
moveset choice kept every shield on the plain moveset ("what sets the
flags is not known yet").

## Evidence

REA (Ghidra and the objdump listing of `Exanima.exe` 0.9.5):

- The two side starters (main side, motions 5–7; other side, 8–10) take the
  crouch first (`comiss` against 0, a NaN not crouched: the side's first),
  then, for stance class 3, call the parry chooser with the fighter as the
  defender and its target as the attacker when the target is in condition 2
  and the technique word has Ward (`0x4000000`) or Synchrony (`0x2000000`)
  with the start not chaining; for weapon class word exactly `0x90` always.
  The chooser's clip maps by the side tables below (checked instruction by
  instruction: `sub eax,0x15; jle`, `sub eax,1; je`, `sub eax,1; jle`). An
  aimed pick writes the target's motion into the countered-motion word,
  else −1 is written. Without an aimed pick, the last parry pick (never
  cleared; written by the parry check's repick) steers the side while the
  clock is under the last parry check's stamp + 500 (64-bit `jle`).
- Tables (pick index): main side, chooser and shield memory: Left/Right 1,
  Up 0, Left Dn/Right Dn 2; two weapons' memory: Left 1, Up 0, Left Dn 2.
  Other side, shield chooser and memory: Left/Right 2, Up 0, lows 1; two
  weapons' chooser: Left/Right 1, Up 0, lows 2, memory: Right 1, Up 0,
  Right Dn 2.
- The overhead and stab starters write −1, then the target's motion when
  (stance class 3 with Synchrony, else class word `0x90`) the chooser gives
  Left or Left Dn (and Up for the stab).
- The chooser reads the attacker's previous motion word while its state
  word carries the continuation bit `0x10000`; the starters write that word
  only when chaining.
- The AI's side attack wrapper calls the main-side starter with no chain,
  so its clip is the starter's pick.
- The technique pass sets `0x1000000 << (id − 1)` for the Shields skill's
  techniques 1–6; the moveset choice takes `0x300` (Shield C2) on
  `0x4000000` (Ward), else `0x200` (Shield C1) on `0x2000000` (Synchrony).
- The weapon body's impact callback calls the immediate counter only where a
  hard hit would interrupt the swing (state & `0x10005` = 1, bit 2 clear,
  children, past 0.6, the primary chain) and for class word `0x90` past 0.4;
  a counter that starts replaces the interrupt; the counter runs the other
  side's starter as a chain.

## What Changes

- `exa_combat::attack`: `Aim`, `FoeParry`, `aimed_side`, `countered_by`,
  `Attacker::pick_aimed`, `Attacker::aim`, `Attacker::foe_parry`.
- `exa_combat::parry`: `chooser_clip`, `aim_foe_parries` (each fixed step,
  before the attack starts), `ParryState::last_pick`; the parry check's
  chooser reads the chained-from motion (`Attacking::continued_from`).
- `exa_combat::write_countered` and `AttackStart::countered`: every attack
  start writes the countered motion.
- `exa_combat::swing::chain_side_attack`: the chain shared by Remise and
  the immediate counter, now started from the weapon-body impact.
- `exa_ai`: side attacks pick through the starter; `exa_game::player`: the
  player's side, stab and overhead starts aim.
- `exa_character::held`: the shield moveset style from the holder's
  techniques.
