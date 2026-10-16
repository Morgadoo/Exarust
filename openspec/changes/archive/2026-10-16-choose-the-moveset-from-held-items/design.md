## Context

How the original picks a character's clips. `humanoid.rsq` holds 32
movesets; each defines some of the 48 actions and names a parent that
supplies the rest ("One Handed" falls back to "Default"). After every
equip change the original runs one routine that reads what the character
holds and builds its motion table from the race's own moveset, a weapon
moveset and an overlay:

1. The weapon class starts as the low 16 bits of the first held item's
   weapon flags. Empty hands are not empty: the unarmed set-up puts both
   hand bodies in the held list, each with a built-in weapon record whose
   flags have no hands bits (0, or 0x40 for the two clawed records).
2. A shield gives "1H & Shield", or "Shield C1" or "Shield C2" by two
   character flags, stance class 3, and the "Torch" overlay when the main
   item is a light.
3. Else a light in the off hand gives "1H & Torch", stance class 4.
4. Else a light in the main hand gives "One Handed" with the "Torch"
   overlay, stance class 5.
5. Else, with something held, by the hands bits (flags & 0xF0):
   one-handed "One Handed" (1), or "2W Light" (6, weapon class 0x90) with
   two held items; two-handed "Greatsword" when the flags are 0x521,
   "2H Light" when kind bit 0 is set (bladed or stick), else "Two Handed"
   (2); polearm "Polearm" (2).
6. Anything else (the fists, or hands bits 0x40 and above) keeps the
   race's moveset, stance class 5, weapon class 0x80, or 0xC0 with a race
   flag. With nothing held at all the class stays 0.
7. The weapon class is masked to its high nibble (flags & 0xF0).

The player's attack, in the behaviour tick's combat branch: a new left
press sets an attack request; a press within 300 ms of the last one adds
the double-click bit and clears the last press. The side is the other one
when the wrapped difference "cursor yaw − body yaw" exceeds 20°, or 15°
when the weapon class is 0. The cursor yaw is that of the vector from the
body to the cursor's point. Positive yaw turns go from +Z toward +X, the
body's left (its right is −X in both engines), so the other side is the
cursor on the left. The executor starts the other side's attack (Combo
1–3) or the main side's (Attack Mid/Down/Up), each picked by the global
32-bit generator (×0x08088405 + 1, index = state·3 >> 32), redrawn while
equal to that side's last pick, which starts at 0 in a new character. A
double click starts Overhead Swing unless a Stab or an Overhead plays.
While no attack is requested, the heading becomes the cursor's direction
once it is more than 30° off it (cos 30° in the routine); while a
request stands (the button held) the heading is the cursor's direction
every tick. The body yaw then turns toward the heading as in exploration.

The research notes, with addresses, are git-ignored:
`context/re/combat-stances.md` §1 and §7, `context/re/player-control.md`
§1.5 and §3, `context/re/combat-look-plan-final.md` (L1, L3).

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Weapon stats:
the flags word), with the rules in [`docs/PLAYER.md`](../../../docs/PLAYER.md).

## Goals / Non-Goals

**Goals:**
- the player and the duellist fight with the moveset their hands pick, in
  and out of combat mode; unarmed, the "Default" punches;
- the original's attack choice on a left click: side, double click,
  random picks without repeats;
- the combat facing the side test depends on.

**Non-Goals:**
- the combat stance and the guard (L2), fists that strike, cooldowns and
  the energy gate (L3), parries (L10), the AI's attack choice (L11);
- shields and second weapons as equipment, overlays over a shield
  moveset, non-human armed movesets.

## Decisions

- **The rule is a pure function in `exa_character::held`.** `Hands`
  describes the hands as the rule reads them (the main item's flags and
  whether it is a light, a second weapon, a shield style, an off-hand
  light); `choose(hands, base, claws)` returns `HeldMoveset` (moveset
  name, `StanceClass`, weapon class). Only bodies that carry `Hands`
  follow it: the player (required by `PlayerBody`) and the duellist. The
  character view keeps the moveset its command line names, and level
  characters keep their race's moveset until equipment exists for them.
- **Who fills the hands.** `equip_outfits` sets the main hand to the
  wielded weapon's item flags when it attaches the weapon (the item given,
  else the first item showing the model, else the plain sword, 0x11).
  Lighting the carried torch puts it in the main hand as a light (the
  torch item's flags 0x13, a one-handed stick) or sets the off-hand light.
  A system after dressing recomputes `HeldMoveset` whenever `Hands`
  changes and writes the moveset into the `Animator`; the clip playing
  goes on in the new moveset, as the original's rebuilt motion table does.
- **Overlays.** An Exarust moveset name is one moveset and its parents.
  "One Handed" with the "Torch" overlay is the "Torch" moveset (whose
  parent is "One Handed"). A shield moveset with the overlay cannot be
  named; it gives the shield moveset until Exarust holds shields.
- **Combat mode no longer changes the moveset.** The player's exploration
  steering and Tab only change the action; the get-up's follow-up stance
  comes from the combat flag for the player instead of "the Default
  moveset means out of combat".
- **The attack choice lives with the player** (`exa_game::player`):
  `AttackChoice` keeps the last click time, a generator state and each
  side's last pick. The generator is per player, seeded 0: the original's
  is shared by everything random, so its sequence cannot be matched; the
  rule (uniform among the two clips other than the last) is.
- **Starting an attack** keeps Exarust's existing gate for the sides (no
  attack running and a looping clip) until L3 brings the cooldowns and
  the energy gate; the overhead may cut a side's attack short.
- **Facing.** `player_combat` (every frame) sets the heading by the 30°
  rule or to the cursor while the button is held; `steer_player` (every
  83 Hz tick) turns the body with `body_turn`, without the keyboard
  factor. The footwork steps along the cursor's direction (`aim`), which
  Tab resets to the heading so an unattended run without a cursor steps
  along the facing.
- **Debug helpers.** `--press` accepts `LMB`, `RMB` and `MMB`, pressed
  and released like keys; `--cursor x,y` inserts `ScriptedCursor`, which
  the cursor's ground point uses instead of the window's pointer.

## Risks / Trade-offs

- [The facing now lags the cursor by up to 30°, which changes the feel of
  combat mode] → it is the original's rule, and without it the side test
  can never see the cursor to the left of the facing.
- [Another lane adds a cursor and mouse presses for the world cursor
  (L4)] → the command-line syntax is the same (`LMB@frame`,
  `--cursor x,y`); at merge one implementation is kept.
- [Unarmed attacks still cannot hit] → stated in the docs and the spec;
  L3 makes the fists strikers.

## Open questions

- What sets the two character flags that pick Shield C1 and C2 → REA on
  their writers when shields come (not needed now).
- Which race carries the clawed-fists flag → REA on the race object's
  flags with the race lane; humans never do.
- Whether the double click is usable in practice with the 400 ms
  cooldown (the second press must still be held when the cooldown ends) →
  retail request 2 of the combat-look plan, with L3.
