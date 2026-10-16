## Why

Exarust puts a limp body to sleep once every particle has stayed under
1 cm/s for 166 ticks (2 s). The original uses a countdown per body tree
with bands at 20 and 40 cm/s, so a corpse settling at a few cm/s sleeps
about 1.5 s after its last fast motion. Since C4 (`damp-limp-joints`)
gave limp parts the original's weaker resting damping, several exanima01
corpses keep 2–8 cm/s of jitter and never meet Exarust's rule: only 3 of
the 7 sleep, and the rest are simulated for ever.

Phase 3 (Bodies), milestone M3 "standing man". This is side track S1 of
the body-controller plan, after C4 (done). Waking on contact between
bodies needs collisions between bodies (C17), which do not exist yet.

## What Changes

- Each body keeps a sleep countdown, set to 256 when it is woken. Before
  each tick's integration a limp body takes the band of its fastest part,
  by the farthest one of that part's particles moved in the last tick:
  over 40 cm/s the countdown goes back to 256; between 20 and 40 cm/s it
  is lifted to 32 from below, falls by one above 32 and by two at 32, so
  it is held at 30–32; under 20 cm/s it falls by two. It never falls
  below 1. The post-step puts the body to sleep at 1 (countdown 0).
- The first tick asleep takes the body's velocity away (its positions
  become its previous ones) and puts it in deep sleep, where nothing is
  done. A sleeping body is drawn where it lies and is not tested against
  the level.
- A living body is held at 256 every tick: it never sleeps, standing,
  fallen or getting up.
- Waking: going limp wakes a body to 256 (it keeps the living body's
  countdown); a blow (a shove, so every hit) wakes it to 256 as a touch
  from a living body does and drops its stored contacts. An explicit wake
  keeps the contacts, which still hold where the body lies. Level
  contacts never wake a body. The wake of a contact between bodies is
  ready for C17 (the lower countdown takes the higher one).
- Removed: the 1 cm/s for 166 ticks rule.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra): the plain
  body step's speed bands and group counter, the post-step's sleep, the
  sector step's sleeping and deep-sleep branches, every wake helper
  (which raise or set the countdown, which drop the contacts), the living
  limb step, which wakes its tree to 256 every tick, the death routine,
  which leaves the countdown alone, and registration, which sets it to 0
  (`context/re/body-sleep.md`, `body-misc.md` §3, `body-tick-order.md`
  §1, §3, §7, `body-pairs.md` §11.5, `body-contacts.md` §9).
- Exarust runs through `gpu_run.sh` with a temporary sleep probe:
  `play exanima01`, `--limp character`, `--shove 60 play arena`, and a
  forced kill of an `--opponent` duellist.

Out of scope:
- collisions between bodies and their wake (C17): the hook is in place,
  nothing calls it yet;
- placed corpses built in their saved pose with the registration
  countdown of 0, asleep after one tick (S2): Exarust's corpses still
  collapse from standing, so every body is built woken;
- dormant statues and golems, the only living bodies the original lets
  sleep; frozen and placed body states; level props, which the original
  starts in deep sleep and Exarust does not simulate.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: limp bodies sleep by the original's countdown, banded
  at 20 and 40 cm/s, with a sleeping and a deep-sleep tick; living bodies
  never sleep; blows and going limp wake a body.

## Impact

- `exa_physics` (`body.rs`: `Ragdoll::countdown`, `Ragdoll::sleep`,
  `wake`, `wake_to`, `touch`, `equalise_sleep`, the countdown before
  integration, the post-step, the sleeping tick, `step_bodies`; tests in
  `body_sleep_test.rs` and `body_contact_test.rs`)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAN.md`,
  `docs/PERFORMANCE.md`, `crates/exa_physics/README.md`
