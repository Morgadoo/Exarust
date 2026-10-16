## Why

Exarust spawns every character as a full-strength human of scale 1: every
undead of the story levels (each of the 97 living humans is feral, 0.4 to
1.0) stands, holds its head and recovers from a daze like a healthy man;
health ignores the record and the original's race-byte rules; the support
pins (C14), the walk (C18), the weapon record (M21), the damage rules (M29)
and regeneration (M43) have no per-character value to read. The original's
character builder and init derive these at spawn from the record (height,
feral level, saved pools, race byte) and the race.

Phase 3 (Bodies), milestone M3 "standing man", and Phase 4's M4 "first
duel" (health, rule bits). This is change M41 of the master plan
(`context/re/master-plan.md`, lane F, F4; body note B4).

## What Changes

- New `exa_formats::character_stats`: `CharacterStats::derive(record,
  race, spawn)`, the original's spawn rules:
  - body scale min(race scale, 1.5)·(1 − 0.05·height) and size race
    size·(1 − 0.05·height); standing pelvis 109.5 − 11.25·h cm, leg reach
    107.2 − 11.1·h cm, locomotion clock factor 1/(1 − 0.1·h); height and
    feral level kept within −1…1 and 0…1;
  - most energy 1 − 0.1·f, support pins' factor race − 0.25·f², head gain
    1 − f, head muscle factor 1 − 0.8·f, walking arms clamp(0.2 − 0.15·f,
    0.05, 0.2), sprint gait (M40's blend), perception 1 − f/2,
    attack-voice step 1 for the feral;
  - step reach 80 cm·√scale·(1 − 0.3·f)·(1 − 0.2·encumbrance);
  - voice pitch clamped 0.9…1.07 (the hulking ghoul 0.85), voice volume
    scale × race factor; clip-rate base the race's;
  - maximum health 0.25 × race factor with the race-byte rules (ogre 1.5,
    Sir 1.0, golems 0.5 and the boss 1.0, the manipulator 0.375, …);
    yellow and red from the saved pools; a living placement whose record
    saved no red health (the arena's opponents) spawns full;
  - the body rule bits (creature, remedies, fierce, no lasting wounds, may
    rise in the story, red regeneration, steadfast, no red regeneration).
- `exa_physics`: `BodyStats` on `Physical` and the drive: energy recovers
  up to the most energy; the head's pose gain and muscle factor and the
  walking arms follow the character; limbs' factors are 1.25 × scale.
- `exa_combat::Fighter::of_stats` replaces `Fighter::of_race` and
  `of_record`: health from the stats; the fighter keeps its stats.
- On main (after the combat stance, the swing clock and the attack
  executor landed): the daze (E < Emax − 0.05), its pose hold
  ((E/Emax)⁴), the swing clock's energy share and the attack gate
  (0.8·Emax) read the character's most energy instead of 1.
- `exa_world::PlacedCharacter` carries its record; play mode derives
  every placed character's stats, and the player's and the duellist's from
  a new human record.
- `exa-tool characters` prints the stats each record spawns with.

Evidence:
- REA and `exe.asm` on `Exanima.exe` 0.9.5: the character builder (clamps,
  scale and size, standing height, leg reach), the init (most energy,
  reach, pins, rule bits, perception, voice, health and its race-byte
  rules, sprint blend, clip base), its head setup (gain, factor), the body
  tick's walking arms, the energy cap, the spawn's call order (builder,
  then registration and the init, then the armour pass, the reach pass
  and the pool load) and the placement reader that refills an unsaved red
  pool (`context/re/master-body.md` §4.1, §7.2–7.4, §11 and the new §18).
- The installed game: every placed character of the 13 story levels and
  the 21 arena actors, pinned by corpus tests.

Out of scope:
- encumbrance (M42; the reach takes it as an argument, 0 until then);
- consumers that do not exist yet: the support pins (C14), the walking
  gait (C18), the clip clock (M14), the weapon record's size (M21), the
  damage rules' bits (M29), regeneration (M43), voices, perception;
- the render scale: the body scale is a dynamics factor, not a mesh
  scale; height reshapes the body through morphs (M18);
- the per-part leg scale the builder sets from thigh and shin extents, the
  balance factor, the link iteration count;
- the player's and the duellist's own records (they spawn as a new human
  record) and the arena's roster.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: stats derived at spawn, health at spawn and body
  rule bits (new requirements).
- `physical-body`: the most energy, the head's gain and factor, the
  walking arms and the limbs' scale follow the character (modified Muscles,
  Blows and daze, Muscle gains, Arm stiffness, Pose hold).
- `combat`: the attack gate and the swing clock measure the energy
  against the character's most (modified Attack executor, Swing pace).
- `asset-inspection-cli`: `exa-tool characters` prints the stats each
  record spawns with (modified Character inspection).

## Impact

- `exa_formats` (`character_stats.rs`, tests in `character_stats_test.rs`)
- `exa_physics` (`body.rs`, `body_muscle.rs`: `BodyStats`, `Physical`,
  `Ragdoll::with_stats`; tests)
- `exa_combat` (`Fighter::of_stats`, `Fighter::stats`)
- `exa_world` (`PlacedCharacter::record`), `exa_game` (`limp.rs` passes
  the stats to the body), `exarust` (`viewer.rs` derives them at spawn)
- `exa_tool` (`characters`)
- `docs/formats/RFC.md`, `docs/formats/RSQ.md`, `docs/PLAN.md`, crate
  READMEs
