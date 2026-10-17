## Why

Armour weighs nothing in Exarust: the object database's encumbrance byte
is parsed and never summed, so a knight in plate runs, reaches and steps
like a man in his shirt. The original keeps one weight stat, the
encumbrance, which its armour pass sums at spawn (and on every equipment
change) from what the character wears and the shield it holds, less what
its armour and shield techniques take off; the step reach, the run, the
turn, the steps, the get-up clock and the weapon record read it. The same
pass also widens coverage with Deflection, and reads each item's
protection as the item stores it, while Exarust scales it by the item's
condition a second time.

Phase 4 (Combat and items), milestone M4 "first duel" (armour visibly
matters), and the master plan's MS3. This is change M42 of the master
plan (`context/re/master-plan.md`, lane F, F5; body note B8).

## What Changes

- New `exa_formats::encumbrance`: the armour pass's sum, in its mixed
  precision: 0.0025 per unit of each worn item's encumbrance (an unpowered
  golem suit adds 2), less 0.025 for each articulated piece whose coverage
  touches another articulated piece (of any set, through the original's
  neighbour table), less 0.3 per Manoeuvring in Armour tier, nothing below
  0.001, plus the held shield's (2 − weightless) × bulk / 4 less 0.3 with
  Mobility when positive (powered shield-lightening gauntlets make it
  weightless), at most 1; `of_record` (a record's worn items and the shield
  of its first two held items, found in the level's, the actor's or the
  game's database); the factors the encumbrance gives its consumers (run
  cap, sprint clock, turn rate, standing alignment, long steps, footwork
  steps, get-up clock).
- `exa_formats::objdb`: every item body's `Burden` (encumbrance, covered
  bones, articulated set, special kind, powered), `worn_as` (the item a
  garment given by its model is); the held shield's share comes from its
  record (`Item::shield`, whose bytes are now scaled in double precision
  as the original builds them);
  `characters::actor_items` (an actor's own item database).
- `CharacterStats` keeps the encumbrance (`with_encumbrance`) and whether
  Deflection is learnt.
- `exa_combat::damage::armour_layers` reads protection as stored (no
  second condition scaling) and applies Deflection's coverage bonus.
- Play mode: every placed character spawns with its record's encumbrance
  (`exa_world::PlacedCharacter::encumbrance`), the player and the duellist
  with their outfits' (logged); the player's run blend rises at most to
  1 − 0.3 × its encumbrance.
- `exa-tool characters` prints each record's encumbrance and the step
  reach it gives.

Evidence:
- REA and `exe.asm` on `Exanima.exe` 0.9.5: the armour pass (tier and
  Deflection from the Armour techniques, per-item sum, articulated pairs,
  tier subtraction, floor, shield term with Mobility, cap; every constant
  read from the image), its special-item handlers (golem suit +2 unpowered,
  gauntlets' weightless shield), the spawn (held shield attached before the
  armour pass, the reach pass after it), the shield record (bulk byte × 0.01
  in double), every reader of the encumbrance (run cap 0.3/0.4, sprint clock
  0.1, turn 0.6, alignment, long steps 1.5/1.6 − 0.3, get-up 1.5 − 0.5, step
  reach 0.2, the weapon holding pass), the item instance rule that stores
  the condition in a made item's bytes (`context/re/master-body.md` §6.2 and
  the new §19).
- The installed game: the object database (388 item bodies, 14 articulated
  sets), every placed character of the story levels, the 21 arena actors
  with their own databases; the level item instances store their
  condition-scaled bytes (381 encumbrance bytes scaled against 1 as the
  template, 2200 protection bytes against 4); pinned by corpus tests.

Out of scope:
- consumers that do not exist yet, which read the factors when they land:
  the body turn (M1), the sprint (C18), long steps and footwork (C13, C16),
  the get-up clock (C11, M14), the alignment (C18); the weapon record's
  holding pass (M21, merged on main after this branch's base: its holder's
  encumbrance comes from these stats);
- equipment changes (no inventory yet: the sum runs once, at spawn);
- the armour pass's other duties: the second-wind, healing-mask, null-helm
  and necklace items, the body byte `0x1A7` value;
- making items from templates (the condition rule is documented; the
  shipped instances already store its result);
- the light a left hand's item may carry instead of a shield (none of the
  22 shipped shields has a light section).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: the encumbrance summed at spawn and what it slows
  (new requirements).
- `combat`: armour reads stored protection and widens coverage with
  Deflection (modified Armour).
- `asset-inspection-cli`: `exa-tool characters` prints the encumbrance
  (modified Character inspection).

## Impact

- `exa_formats` (`encumbrance.rs` with `encumbrance_test.rs`, `objdb.rs`,
  `characters.rs`, `character_stats.rs`)
- `exa_combat` (`damage::armour_layers`, `damage::coverage`,
  `Fighter::of_stats`), `exa_character` (dressing passes Deflection, no
  condition), `exa_world` (`PlacedCharacter::encumbrance`), `exa_game`
  (`player.rs`: the run cap, a small edit in lane D's file), `exarust`
  (`viewer.rs`), `exa_tool` (`characters`)
- `docs/formats/OTHER.md`, `docs/formats/RFC.md`, `docs/PLAN.md`,
  `docs/PLAYER.md`, crate READMEs
