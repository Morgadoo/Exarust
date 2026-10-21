# Proposal

## Why

Review of the arena career lane. The return to the hub on a tournament day
settles the events before the first open entered one a second time, with a
rival drawn from five companies instead of six; Exarust only had the
tournament start's pass. The hub arrival's "physician's mark" was a guess
written as fact: the original removes a weapon-kind object lying at a fixed
hub spot, the same call the physician's hire makes.

Phase 7, milestone M7 "arena season" (`docs/PLAN.md`).

## What Changes

- `tournament::settle_after_return`: the return's pass, a rival of
  `2 + high(roll × 5)` (2 to 6), beside the start's `settle_unentered`
  (2 to 7), both one loop.
- The hub arrival's flag is `clear_physician_spot`, documented as the object
  at (−2100, 0, 900) cm, its meaning labelled as not traced.
- `docs/formats/OTHER.md` loses a stale duplicate "Character creation"
  paragraph (with a retail address) that a cherry-pick's conflict resolution
  had brought back.

## Evidence

REA/Ghidra on Exanima.exe 0.9.5: the return-to-hub routine and the
unentered-event pass it calls on a day divisible by 7 (`IMUL RCX, RCX, 5`
where the start's pass multiplies by 6), the hub arrival routine and the
object lookup it calls (kind `& 0x7000 == 0x4000`, x and z within 10 of the
spot's floats). Notes: `context/re/arena-manage.md`.

## Out of scope

Wiring the return into a running hub (there is no hub screen yet); what the
removed hub object is.
