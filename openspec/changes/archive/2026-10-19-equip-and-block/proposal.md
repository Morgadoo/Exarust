# Proposal

## Why

Phase 4 still has no equipping, no container transfer, and no block or parry. A blow always takes the flesh wound. `docs/PLAN.md` §7 lists inventory and then blocking and parrying next.

## What Changes

- Equipping takes off every worn item whose occupancy overlaps the new one. Displaced braies are dropped. Other displaced items return to the pack. A human left bare at the groin wears braies again.
- A container and a pack exchange an item by moving its id.
- A parry stops the blow. A shield that is not parrying deals only the yellow leak.

## Capabilities

### New Capabilities

- `carried-items`: equipping, underwear and container transfer.

### Modified Capabilities

- `combat`: a guard changes the blow the hit applies.

## Impact

- `exa_items::carry`, `exa_combat::damage::guard_outcome`, the hit path.
- `docs/PLAN.md`, `docs/formats/OTHER.md`.
- Evidence: the occupancy and underwear notes, and the clash and shield-block notes, already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: inventory windows, parry clips, shield bodies, grabs, and the striker's recoil animation.
