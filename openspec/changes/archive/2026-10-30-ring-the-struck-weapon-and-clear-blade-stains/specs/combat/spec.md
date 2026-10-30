## ADDED Requirements

### Requirement: Wound rings
A wound on flesh, a turned one too, SHALL play the hit sounds on every contact: the weapon's set (its haft class's shared set when the haft took it, else its own sound, none without one), stamping the weapon's sound clock at the game clock plus 200 ms less round-half-even(80·volume); and each layer above the epsilon from the layer table by its class: flesh `FlsCol`, cloth `HitClt`, leather `HitLtr`, mail `HitChn`, plate `HitPlt`, the sixth class `HitHbr`, golem `HitGlm`.

#### Scenario: Bare flesh
- **WHEN** the one-handed sword cuts a bare limb
- **THEN** the sword's own set rings at the blow's volume and pitch, and `FlsCol` at 1.5 times the volume times half its loudness, pitch 0.4·pitch + 0.7

### Requirement: Struck weapon ring
A clash's first ring of a swing SHALL also ring the struck weapon: its record's class set (bits 16–17: the wood, metal or third haft set) when the record has class bits and the blow's point, projected on the weapon's long axis from its grip ((y + x) + z) over its length, lies strictly below the record's split; otherwise, a NaN projection included, its own sound, and nothing without one. It plays at the blow's volume, with no share, and 1.2 times its pitch in double precision.

#### Scenario: Low on a metal haft
- **WHEN** a sword clashes on a weapon whose record has the metal class, half way up to its split
- **THEN** the struck weapon plays `WH_HaftMA` at the striking sword's own ring's volume and pitch

#### Scenario: High on the blade
- **WHEN** the same clash lands half way between the split and the tip
- **THEN** the struck weapon plays its own sound

### Requirement: Struck weapon sound clock
The struck weapon's ring SHALL stamp its sound clock at the game clock plus 200 ms less round-half-even(80·volume), whether or not it rang. A later clash in the same swing SHALL ring neither weapon.

#### Scenario: Two clashes in a swing
- **WHEN** a swing clashes twice on the same weapon
- **THEN** each weapon rings once
