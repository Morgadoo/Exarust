## Why

PLAN Phase 6 (thaumaturgy, Displacement) and Phase 5 (NPC combat). Round 103
traced the Summon (`TPwrDispSummonDemon_I`) and round 104 gave the demon
(`demon a01`, race byte 11) its body, but casting the summoning spawned
nothing. The harvester also held in combat idle at exanima67's rocky spots.

## Evidence

REA (Exanima.exe), read against the assembly:
- The summon's activation: reads `demon a01` from the character database,
  registers it under a fresh id, places it at the picked spot facing the
  caster (`caster − place`, level, rsqrt-normalised; at or below 1 cm² of
  floor length the caster's own forward and yaw), AddRole 33 (Activity
  Wait), familiarity 75 both ways and relation role 28 (Ally) both ways.
- The deferred lookup and the arrival: once the body exists and the place is
  known, the body is placed, held (behaviour statue byte, sleep word 2,
  condition 0), the caster's power memory keeps the demon's id, the shell
  (resting, open 3000 ms, close 1000 ms, opacity 2, texture scale over the
  demon's size) and the tendril set go on the demon, `thmdsp_Smn` plays at
  0.4 and `roll·0.2 + 0.9`, the held phase lasts 2000 ms.
- The control: phase 3 → 4 opens the shell and moves the summon's own
  four-entry phase table; phase 4 → 5 releases the demon (statue byte off,
  roles re-applied, body woken for 256) and closes the shell; 1000 ms later
  the summon ends. Phase 0x10 is the Long Door's grid query.
- The AI's summoning advice refuses once the power memory holds a creature
  list; nothing checks that the creature lives, nothing sends the demon away
  after a time or at its summoner's death, and nothing makes it follow.
- The contact pass: a statue with no sense of its own does not perceive; its
  contact's concern and hostility are zeroed.
- The contact integration publishes the watched point as the target's chest
  across the ground at the target's ground height; the chase check measures
  that height against the fighter's own ground.

## What Changes

- exa_game: the Summon class (pick, ask, arrival, held, fade, release,
  tail); `thaum::summon_live` (the demon request, the arrival's roles,
  relations, power memory, vanish, tendrils and sound, the release); the
  summon's phase table; the shell's own open and close lengths, opacity and
  texture scale; the AI's summoning advice reads the power memory.
- exarust: the demon is built through the placed-character path; the
  debug flag `--hurt-casters r`.
- exa_ai: a statue without a sense perceives nobody; the watched point sits
  on the foe's ground; a stuck-reason and chase trace for `EXA_DUEL_TRACE`.

## Out of scope

The held body's sleep word, the fade's caster-body gain, the summon's light,
the harvester's chase walk (its body's recovery-step flag stays set at the
rocky spot, so the AI driver never starts the route).
