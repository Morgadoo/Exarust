## Why

The original's arena screenshots show blood where the blows landed: down
a belly, on chests, arms and braies, and on corpses. The previous change
(`shade-and-paint-blood`) gave Exarust the blood channel, the painter and
the shading, but only test wounds paint it: blows leave no blood.

In the original a blow bleeds when the victim's race bleeds and the
wound's size R (the lasting damage on the part, its multiplier applied,
before the red bar's 0.75) exceeds 0.05. The wound waits on the victim,
anchored to the striking weapon, and is painted seven body ticks later,
smeared along the weapon's follow-through, with radius 30·R + 10 cm, on
corpses too.

Phase 5 (Visual and audio fidelity), milestone M5 "looks and sounds like
Exanima", item "Effects: blood sprays and spatter decals". It also serves
Phase 4's M4 "first duel".

## What Changes

- **Bleeding blows** (`exa_combat`): `Outcome::wound` (R) next to the red
  damage; `Fighter::bleeds` (true for humans, from `races.rdb` flag bit 0
  through `Fighter::of_record` for placed characters); a blow on a fighter
  that bleeds with R > 0.05 queues a wound.
- **Wound queue** (`exa_combat::wounds`): `WoundQueue` on every fighter,
  seven slots, a new wound dropped while seven wait; each record holds the
  striking body (`Hit::striker`, the blade), the contact point as weights
  in its tetrahedron, the world point, a delay of 7 ticks and R. Every
  body tick counts the queue down; the first record whose delay is up is
  sent as a `Bleed` (one a tick, corpses too) with the striker's travel
  since the blow and the radius 30·R + 10 cm.
- **Painting** (`exa_character::blood`): each `Bleed` becomes a
  `PaintWound` (centre P − D/2, the smear along D, min-only), on the same
  targets as test wounds: the skinned body, every skinned garment, the
  meshes under the head (helmets, hair); never held items or rigid items
  on other parts. Nothing paints the striker: a fist stays clean.
- **Debug helper**: `--click frame,...` clicks the left mouse button at
  frames after the load, so the armed player attacks in unattended runs.
  Each hit's log line names the struck body, the body tick and R.
- Docs: `docs/RENDERING.md` (Blood), `docs/formats/OTHER.md` (Damage),
  `docs/DEBUGGING.md`, `docs/PLAN.md` (Phase 5), crate READMEs.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, `exe.asm`): the
  damage routine's blood branch (race flag bit 0, R before the 0.75 against
  0.05 strictly, the queue call with the striking body and R), the queue
  (seven records, full → dropped, delay 7, the weights of the contact
  point in the striking body's tetrahedron), its consumer in the body tick
  (every delay counted down, the first painted when its delay is up, D =
  the weighted point now − the contact point, radius² = (30·R + 10)²), and
  the striking-hand slot that keeps fists clean
  (`context/re/wounds-and-blood.md` §1.1–1.4, refreshed this change).
- Corpus: `races.rdb` flag words (pinned by the previous change's test).
- Exarust in game through `gpu_run.sh` (arena duels), compared with the
  retail shots in `context/re/refs/blood/` and `combat-live/`.

Out of scope:
- blood sprays (L9), blade stains (L8), floor blood;
- the attacker's own blood when it strikes with a body part of its own
  (creatures' slot), the collapse before death (L5), fists that strike (L3);
- blood across re-dressing and saves (Exarust never re-dresses a character
  yet; retail requests R2, R3).

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `blood`: wounds from blows: which blows bleed, the wound queue and its
  timing, the smear from the striker's follow-through and the radius.
- `world-viewer`: the `--click` flag.

## Impact

- `exa_combat` (`damage.rs`: `Outcome::wound`; `lib.rs`: `Fighter::bleeds`,
  `Fighter::of_record`, `Hit::striker`, `CombatSystems`, the queueing in
  the blow reactions; new `wounds.rs`)
- `exa_character` (`blood.rs`: `Bleed` → `PaintWound`; dev-dependency on
  `exa_physics` for its test)
- `exarust` (`main.rs`: `--click`; `viewer.rs`: the clicks, placed
  characters' fighters from their race record)
- `docs/RENDERING.md`, `docs/formats/OTHER.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, the READMEs of `exa_combat`, `exa_character`, `exarust`
