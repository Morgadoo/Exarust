# Design

## Foe identity

A character in Exarust is a root (capsule, transform) with a model child
carrying the fighter, the roles and the id. Duellists aim at the root; the
brain's perception walks fighters. The brain therefore names its foe by the
fighter at or under the duellist's target, and the duel tick keys the foe's
contact by the same fighter, so one contact carries the perception, the
integration's hostility and concern, and the combat state's reads. A target
that is itself a fighter (the tests' flat entities) is kept.

## Arena roles

| who | original | Exarust before | now |
|---|---|---|---|
| practice player | cleared, Unaligned | Unaligned | Unaligned |
| practice opponent | Unaligned | Unaligned | Unaligned |
| career player | cleared, Faction 1 | Unaligned | Faction 1 |
| two-sided opponent | Faction 2 | Faction 2 | Faction 2 |
| free-for-all opponent | Unaligned | Unaligned | Unaligned |
| company entrant, staff | Faction 1 | Faction 1 | Faction 1 |

Shipped attitudes: Unaligned and both factions rate every other arena side
−100 at tolerance 0; a faction rates itself +100 at tolerance 100; none rates
Player, Human or a background role.

## Combat step

The carry walk zeroes the capsule's velocity because the body's push moves
it. A footwork step does not locomote (no push), so the capsule keeps the
step's velocity and the body is carried with the capsule, as a duellist's
step already does.

## Open questions

- The original's combat step is a foot step of the balance; porting it
  replaces the interim capsule step (later change).

No docs/formats page is touched (no data format).
