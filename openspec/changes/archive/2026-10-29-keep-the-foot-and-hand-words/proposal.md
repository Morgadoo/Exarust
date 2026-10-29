# Proposal

## Why

TMotilePart was rated 55%. A check of death's lower-leg loop against the
instructions showed that its strength-1 write goes to each lower leg's
limit link, not to a held item's soft weld, so the corpse weld rule was
invented. The foot's thin-plate level bit was taken from "the body has
muscles", while the original keeps it as a word the leg routine sets and
the collapse clears. The hands' contact words, the gobbler's collapse
slack, a held shield's own words and the revive of a corpse had no rule
or no caller. The motilepart9 lane adds them.

## What Changes

- Death no longer stiffens a held item's soft weld; a corpse keeps its
  items on unchanged grip links.
- The foot's thin-plate bit is a stored word: the leg routine sets it on
  each lower leg's first child every living tick that is not a collapse;
  the collapse's start and death clear it with the strike-chain bit.
- A gobbler's collapse gives each femur's limit link 10 cm of slack.
- The hands' contact mobility follows the equip refresh (armed: the base;
  unarmed: half, a quarter with heavy claws), a heavy-claws attack start
  quarters it, and an unarmed collapse puts it back at the base.
- A held shield's body takes the held and mark bits and a link mobility
  equal to its contact mobility.
- The revive of a corpse and the knock-out's character words are tested
  rules (no running caller).

## Phase

Phase 4, Combat and items: held items and their grips (docs/PLAN.md).
