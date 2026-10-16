## Context

The original keeps one sleep countdown per body tree (every body of the
tree holds the same value; the root applies the changes). In the plain
step, which runs props and a dead character's parts, each body measures
the largest squared displacement of its four particles over the last
tick and picks the same band as its velocity retention, strictly over
(40/83)² or (20/83)² cm². A fast body asks the tree to wake to 256. A
slow one asks for a wake to 32 when its countdown is under 32, and for a
decrement of 1 when it is over 32; at exactly 32 it asks nothing. The
root then wakes the tree (a negative request) or decrements it by the
request, 2 by default, never below 1, and resets the request to 2. The
post-step puts a body whose countdown is under 2 to sleep: countdown 0,
the asleep bit, its displacement records zeroed. The sector step rolls a
sleeper's particle history once without moving it (so its velocity is
gone) and sets the deep bit, after which only its tick stamp changes.
Sleepers are skipped by integration, the iterations, level detection and
friction; pair detection still finds them.

A living character's root wakes its tree to 256 at the end of every limb
step; only dormant statues and golems count down instead. Death changes
the condition and the iteration count but not the countdown, so a body
killed in play starts at 256. Registration sets the countdown to 0, so a
placed corpse sleeps after its first tick; the living builder wakes its
body to 256.

Wakes: the explicit helpers raise (or set) the countdown and clear the
asleep and deep bits but keep the stored level contacts; the pair
contact's equalisation (the lower countdown takes the higher) also puts
the sleeper back in the solver list and drops its level contacts. Level
contacts never touch the countdown. The research notes, with addresses,
are git-ignored: `context/re/body-sleep.md` (this change's answers),
`body-misc.md` §3, `body-tick-order.md` §1, §3, §7, `body-pairs.md`
§11.5, `body-contacts.md` §9.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the original's countdown, bands and timing for limp bodies;
- the sleeping tick that takes the velocity away, then deep sleep;
- living bodies held awake;
- waking on a blow and on going limp, and a ready hook for contacts
  between bodies.

**Non-Goals:**
- collisions between bodies (C17); placed corpses in their saved pose
  with countdown 0 (S2); dormant statues; frozen and placed states;
  simulated level props.

## Decisions

- **One countdown per `Ragdoll`.** A `Ragdoll` is the original's tree,
  so the shared value is a field (`countdown`, an `i32` in ticks) and
  the per-body requests collapse into the band of the fastest part
  (`count_down`: fast > slow > resting, the same outcome whatever order
  the parts come in). Exarust applies them in the same tick. In the
  original a part stepped after its root has its request applied one tick
  later; the order in which a tree's bodies are stepped was not traced.
- **Before integration.** `tick_countdown` runs at the start of `step`,
  on the last tick's motion (`positions − previous` before they roll),
  as the original's step does; the band helper `speed_band` is shared
  with the limp retention, in cm per tick.
- **State.** `Sleep::{Awake, Asleep, Deep}`. The post-step sets
  `Asleep` at a countdown of 1; the next tick (`sleep_tick`) copies the
  positions into the previous ones and sets `Deep`; a deep sleeper is not
  touched at all (no change detection). `Ragdoll::tick` honours the
  state, so a body ticked alone sleeps as one in the world does.
- **Living bodies.** A body with a `Drive` has its countdown raised to
  256 before each integration, so the post-step never puts it to sleep.
  Fallen and getting-up bodies keep their drive and stay awake, as the
  original's knocked-out characters do.
- **Built woken.** `Ragdoll::new` starts at 256. The original's
  registration value 0 would freeze Exarust's corpses standing, since
  they still collapse from the Idle pose; S2 builds them in their saved
  pose and can start them at 0.
- **Wakes.** `wake_to(n)` raises the countdown and wakes a sleeper with
  its stored contacts (they still hold where it lies); `wake` is
  `wake_to(256)`, used by `with_drive` and `go_limp`. `touch(n)` does the
  same but a sleeper drops its contacts, as the pair contact's wake does;
  `shove` (every blow, so every hit) is `touch(256)`, the countdown of
  the living body that strikes. `equalise_sleep(a, b)` is the pair
  contact's equalisation, for C17.
- **Units.** Speeds are compared in cm per tick (40/83 and 20/83) on
  displacements stored in metres, through `speed_band`.

## Open questions

| question | how it is answered |
|---|---|
| Countdown start | REA: registration sets 0, the living builder wakes to 256, death keeps it (`body-sleep.md` §1). Exarust builds every body at 256 until S2. |
| Speed measure | REA: the largest particle displacement of the last tick, per body, strict bands shared with retention (§2). |
| The 20/40 cm/s bands and the hold at 30–32 | REA: the slow band's two requests and the root's default decrement of 2 (§2). |
| What sleep does to contacts | REA: kept but unused while asleep; dropped by the pair contact's wake only (§4). |
| Deep sleep | REA: the second tick asleep sets the deep bit; only the tick stamp changes after (§3). |
| Which events wake a body | REA: pair contacts (equalised), explicit wakes (knock-out, revive, statues, grabs), never the level; living bodies are woken every tick (§5). |
| Do living bodies sleep | REA: no, except dormant statues and golems (§5). |

## Risks / Trade-offs

- [Corpses held between 20 and 40 cm/s never sleep] → bodies jittering
  that fast (corpses spawned standing on cots) stay awake, as they would
  in the original; S2 lays them in their saved pose.
- [Sleeping sooner freezes a slow slide] → a body sliding under 20 cm/s
  stops 1.5 s after its last fast motion, as in the original.
- [No body contact wakes a sleeper yet] → a living character walking
  into a sleeping corpse does not wake it until C17; today bodies do not
  touch each other at all.
