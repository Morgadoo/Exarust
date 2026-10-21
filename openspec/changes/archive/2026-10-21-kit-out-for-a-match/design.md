# Design

## Context

Before a match every participant's loadouts are checked and copied, and
the copy is dressed in the loadout of its rank capped by the match's tier.
Rule page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Arena
tables).

## Goals / Non-Goals

**Goals:** the choice, the repair, the rank-up copy and the value as plain
functions with tests.

**Non-Goals:** dressing a body, the event weapon, the loadout window.

## Decisions

1. A Captain's wing at tier 0 or 1 names a rank below 0; the original then
   reads before its first loadout. Exarust returns that rank and leaves the
   read to the caller.
2. The repair trims all six loadouts (the candidate's draw trims only those
   below its level; the repair does not depend on a level).
3. The fighting skill word is identified by its range and the body
   generator's use; the identification is labelled as inferred.

## Open questions

- Which record word the match copy's "fighting skill" is: confirm with the
  record layout when the arena copies characters (REA on the copy routine).
