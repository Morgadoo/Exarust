## Context

M39 read the records whole (the Manoeuvring tier, Deflection and the
shield perks among them) and M41 derived the spawn's stats, leaving the
step reach's encumbrance as an argument at 0. The master plan
(`context/re/master-plan.md`, F5) makes this change the one that sums the
encumbrance, so the reach, the weapon record (M21) and the movement
consumers (M1, C11, C13, C16, C18, M14) read one value.

In the original the spawn attaches the held items (a shield among them)
right after the character builder, then runs the registration and the
init, the armour pass, the held-item pass, the reach pass and the pool
load; the armour pass also runs on every equipment change. The research
notes, with addresses, are git-ignored: `context/re/master-body.md` §6.2
and the new §19 (this change's reading and corrections),
`combat-damage.md` §3 (the armour layers), `master-movement.md` §5–6 (the
movement consumers), `master-plan.md` F5.

The format pages this change extends are
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Armour, a new
Encumbrance section) and [`docs/formats/RFC.md`](../../../docs/formats/RFC.md)
(Stats derived at spawn).

## Goals / Non-Goals

**Goals:**
- the armour pass's encumbrance, in one Bevy-free function with the
  original's order and precision, from a record and the databases its ids
  name;
- every character spawning with it, and the consumers that exist reading
  it: the step reach and the player's run blend;
- the factors every later consumer needs, each pinned to a number;
- the armour layers as the same pass fills them: stored protection,
  Deflection's coverage;
- unit tests pinned to the notes' numbers and corpus tests over every
  placed character, every arena actor and the outfits Exarust gives by
  model.

**Non-Goals:**
- consumers that do not exist yet (turn table, sprint, steps, get-up
  clock, alignment, the weapon record's holding pass on this branch);
- equipment changes, item instancing, the pass's other special items.

## Decisions

- **Where the sum runs.** At spawn, as the original's spawn runs its
  armour pass after the init: `exa_world` sums each placed character's
  encumbrance where it resolves the record's ids (level database or the
  game's), the viewer the player's and the duellist's from their outfits'
  items (the first painted item worn as each model, the rule dressing
  already uses), and `CharacterStats::with_encumbrance` keeps it. Summing
  at dress time instead would miss the held shield and items without a
  worn model; the dressing step keeps only the armour layers.
- **Items, not garments.** The sum reads every worn item with a body
  (`objdb::Burden`), in the record's order, skipping ids that find
  nothing, as the pass does; the shield is the last of the first two held
  items that is a shield and not a weapon.
- **Articulated pieces.** The pass compares only the pieces' coverage
  masks through its neighbour table, never their set ids, so pieces of
  different sets touching count too; the table's left humerus lists the
  right shoulder. Both are kept as the original has them and pinned by
  tests.
- **Precision.** Each step of the sum runs in double and is stored as a
  float; the golem suit's +2 and the shield term are float; the floor
  compares in double. The tier test pins the float left of 0.3 − 0.3
  (1.2e-8) being floored to 0.
- **Condition.** The pass reads an item's protection and encumbrance as
  the item stores them. The condition is applied when an item is made from
  a template (protection × c rounded, capped 160; encumbrance ÷ ((1 −
  0.15·d/255)·(0.85 + 0.3·q/255)) rounded). Of the shipped level instances
  matched to a game template by name whose condition changes their
  encumbrance byte, 381 store it scaled, 1 as the template and 27 neither
  (another template of the same name, inferred); of their protection
  bytes 2200 are stored scaled, 4 as the template, 132 neither.
  Exarust scaled stored protection again; `armour_layers` no longer does.
  Garments given by model are the game's templates, read raw, as a record
  wearing a game id directly is.
- **Factors for later consumers.** `run_cap`, `sprint_clock`, `turn_rate`,
  `standing_alignment`, `long_step`, `footwork_long_step`, `get_up_rate`
  live beside the sum so each consumer change reads the original's
  constant from one place.
- **The run cap now.** Exarust's interim run blend is the original's (it
  rises 0.05 and falls 0.015 a tick), so its cap 1 − 0.3·encumbrance
  applies as is (no sprint latch: Exarust has no sprint).

## Risks / Trade-offs

- Equipment does not change yet, so the value is fixed at spawn; the sum
  is a pure function and can run again when equipping lands.
- Removing the second condition scaling changes every placed character's
  protection by its items' condition factors (0.48–1.3); the stored bytes
  are what the original reads.
- On main the weapon record's holder takes the encumbrance (M21): when
  merged, feed `Holder::encumbrance` from `Fighter::stats.encumbrance`,
  and keep one of `weapon_record::step_reach` and
  `CharacterStats::step_reach`. Done in the merge: the shield's share
  reads main's `Item::shield` (`ShieldRecord::encumbrance`), whose bytes
  are now scaled in double precision as the original builds them, in
  place of a second bulk field.

## Open Questions

- What the AI reads the encumbrance for (its footwork steps use the
  factor; nothing else traced): answered when the AI's combat state is
  ported (L11, M11), from its readers in `exe.asm`.
- The light a left-hand item may carry instead of a shield: none of the
  22 shipped shields has a light or weapon section (corpus); revisit with
  inventory.
- The few shipped level instances whose bytes match neither their
  template's nor its scaled ones (27 encumbrance bytes): the pass reads the
  stored byte, so they change nothing here; telling their templates apart
  needs the instance's template link (inventory work).
