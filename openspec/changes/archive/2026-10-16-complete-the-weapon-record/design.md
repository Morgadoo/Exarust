## Context

The original keeps a 28-element record (0x70 bytes) on every body that can
strike: a held weapon, a shield (a 16-byte record) or a hand. It is built
once from the item's weapon section and rebuilt in part on every equip
change.

1. **Build** (from the section, W[i] = element i): flags & 0x0603FFFF;
   the striking side's type code; the striking part's start, its clamped
   copy (0.7 below 0.15, at most 0.99) and its end, ×0.005 of the last
   part; weight and balance /100; impact (1 − b)·0.6 + 0.4w + 0.1 (only
   0.4w + 0.1 above balance 100) + (impact byte − 127)/100; slash, crush,
   pierce, thrust 0.5·(x/100)^0.7 in double precision, the first three
   from the flipped block when the weapon is flipped; handling
   0.8 − (w′ + 1 − b)/8 with the first build's w′ = (1 − u/2)·w; second
   hand (byte 0x40 − 128)·0.00156 + 1; the hit sound class. Armed, the
   length W21 sums the box tops (centre.y + half extent.y, cm) of the
   weapon's node and of every rigid node down its first-child chain; the
   sweet spot W22 adds whole parts while the next one is rigid, then
   (start + 0.6·(end − start)) of the part it stopped at. Unarmed, both are
   |centre.x| + half extent.x of the hand, and W23 is the end byte.
2. **Holding pass** (encumbrance g, size s): held weight w8 and balance
   w9 (the half-sword hold: (2 − w)·w and 0.7·b), w8 × (1 − u) with u the
   weightless item's share; h = (1 − w9)·0.6 + 0.4·w8 + 0.1 (0.4·w8 + 0.1
   above balance 1); recoil W17 = g/2 + h; k = ((1 − g) + w9)·g/2 +
   w9·(1 − g); swing shaping W18 = 1 − k; blend W19 = round((w8 + 1 − k)·10);
   arm stiffness W20 = 0.7 − 0.3·W17, × (0.7 − 0.2·w9) for a two-hander out
   of the half-sword hold, × 1.5 for the greatsword 0x521, 0.5/s unarmed;
   swing speed W15 = (1 − w8)·0.15 + 1.05, × 1.2 for the human fists; AI
   swing turn W27 = min(0.4·W18 + 0.3·w8 + 0.4, 1.5), unarmed × 1.5 for
   claws else × 0.5; up to 124 cm W25 = 1.5·√((250 − L)·0.004) and
   W26 = (250 − L)/180, else 1 and 0.7; W26 × (1 + d/2), d = w9 − w8 (halved
   when negative); unarmed W25 × 0.7·√scale and W26 × 0.6/scale; armed
   reach W23 = 60·s + W22; the mass M = heavy·w + (1 − w)·light from the
   table (1h, 2h, polearm × normal, ogre, demon), spread half evenly and
   half by the parts' own masses, the weapon's part × s.
3. **Equip refresh**: every held record gets the holding pass with the
   character's g and s; the reach is the largest W23; W19 =
   round(W19 × race 0x90) and W27 × race 0x88; the AI's thrust preference
   is clamp(round(W14/(W11 + W12 + W13)·5) + 1, 1, 9), 10 without edges,
   0 holding nothing; unarmed, the reach is race 0x84 and the hands weigh
   2× their part mass (4× for the heavy claws); race index 6 and 7 (the
   ghouls) set every W20 to 1, race index 5 (the Fael) halves W27.
4. **Unarmed set-up**: both hand bodies become the held list. The right
   hand's section by race index (8 heavy claws; 6, 7 claws; 4, 5 heavy
   claws and the heavy state; else fists), then the left hand's by the
   race's word at 0x58 (0x100 claws; 0x20100 heavy claws and the heavy
   state; else fists).
5. **Shield**: SW1..3 = section `0x8000` bytes 4..6 × 0.01; on attach
   parry 0.5 + 1.5·(1 − SW1)² (SW1/2 with Mobility), leak 1 − SW2/2,
   stamina cost = leak (× 2/3 with Endure); the encumbrance term
   (2 − u)·SW3/4.
6. **Step reach**: 80·√scale·(1 − 0.3·feral, when feral ≥ 0)·
   (1 − 0.2·encumbrance).

The research notes, with addresses, are git-ignored:
`context/re/combat-damage.md` §1 and §3, `context/re/master-combat.md`
§3.2, §4 step 13, §10, §11.1, §14, `context/re/master-body.md` §6.2 and
§7.2, and the master plan's entry for M21; this change appends its
corrections to `combat-damage.md`.

The format page this change extends is
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Weapon stats,
now Weapon record), with the node box in
[`docs/formats/RFC.md`](../../../docs/formats/RFC.md).

## Goals / Non-Goals

**Goals:**
- the whole record, its holding pass and the equip refresh as pure,
  Bevy-free functions, each rule pinned by a unit test;
- the weapon's length measured as the original does, the fists and claws
  through the same builder, the shield record and the mass table;
- in game, held weapons and the duellist's reach on the new record.

**Non-Goals:**
- anything that consumes the new fields (L3, M22, M24, M25, M26, M27,
  L11, C15, C16) and the characters' own size, encumbrance and feral
  level (M41, M42).

## Decisions

- **The record lives in `exa_formats`.** It is data derived from data,
  needs no engine, and the corpus tests run on it beside the parsers.
  `exa_combat::damage` re-exports it and its flags; the damage routine
  reads the same field names as before.
- **The original's units.** The record keeps cm, like the damage routine;
  the game converts at the boundary (`reach_metres`, the blade's length in
  metres).
- **Build, then hold.** `WeaponRecord::build` runs the builder and the
  holding pass with the given holder. The holding pass is idempotent (it
  recomputes everything it writes from the build's fields), so building
  with the character's holder equals the original's build at (0, 1) plus
  its refresh. `equip` repeats the pass before scaling by race, so equip
  changes do not compound.
- **Lengths from the model, not the mesh.** Scene nodes keep their box
  half extents and a flag for a collision body (`0x3DD0B001` after the
  mesh body). `weapon_record::chain` walks the first child (the
  lowest-numbered node whose parent is the current one). The corpus shows
  every plain weapon's box tops at its highest vertex, so a single-part
  weapon keeps the length Exarust measured before; flails gain their
  links. Factory weapons are assembled into one part by the original, so
  their length is the assembled model's highest point.
- **Race fields without touching the race record.** The three weapon
  fields and the clawed-hands word are read from the `races.rdb` payload
  by `RaceWeapons::of_race_record`, beside the human's constant; M40 owns
  the race record and may expose them there. The game uses the human's
  until characters of other races are armed.
- **The shield section is parsed now.** `objdb::Item::shield` holds the
  record; M31 decodes the object database's other sections.

## Risks / Trade-offs

- [Rounding of the blend] W19 rounds ties to even on single-precision
  values; a weapon whose (w8 + 1 − k)·10 lands on .5 may differ from the
  original by one if it computes in another precision. The pinned cases
  are not ties.
- [Unarmed swing pitch] it divides by the hand body's own scale; Exarust
  takes the character's body scale (a part's scale is its character's)
  [inferred].
- [Mass units] the table's units are the simulation's own; nothing in
  Exarust consumes the mass until M24.

## Open Questions

| question | how it will be answered |
|---|---|
| What sets the half-sword hold (RB11)? | REA: indirect byte writes near the character's half-sword byte; until then never set |
| Who reads the clamped start (W3) and the handling (W16)? | REA readers of record offsets 0x0C and 0x40 |
| The factory's head striking extent: is the component box read before or after its deforms? | REA on the assembly's bounds update; until then the template's bytes |
| Do non-human races hold weapons with their own size? | M41 (size, scale) and M48 (non-human bodies) |
