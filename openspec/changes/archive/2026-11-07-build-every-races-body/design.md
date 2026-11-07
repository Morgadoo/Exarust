# Design

## Evidence

- The character builder's model pass (REA) runs for every race: it loads
  the race model, applies the physique, builds the parts and hands the
  character to the init. Its only race-class branches (0x500 to 0x900,
  0x1100, 0x1200) adjust the record, not whether a body is built.
- The character init walks the body's parts for every race (masses from
  the per-bone table, link and contact mobility, the living-limb bit) and
  then branches on the race class: 0x200 the ancient, 0x300 the ogre,
  0x400 Sir, 0x500 the Fael, 0x600/0x700 the ghouls (the chest pins
  re-scaled at half the size), 0x800 the gobbler, 0x900 the manipulator,
  0xE00 the wraith and the golem bit. These are the presets
  `CharacterStats::derive` and `character_muscles` already port.
- `humanoid.rsq` is the only skeleton that ships; its eighteen bone nodes
  carry the body chunks (links, thorax pins, foot contacts).
- Race models (`exa-tool chunks Resource <stem>base.rfc`): human,
  skeleton, ancient, ogre, Sir, Fael, both ghouls, gobbler, manipulator,
  harvester and the five golems carry bone nodes 01 to 18 (the Fael also
  six wing nodes without a bone number); the wraith 01 and 08 to 18; the
  demon 01 to 27.

## Placed races (shipped levels)

| level | races |
|---|---|
| exanima01–03 | Human, Skeleton |
| exanima04 | Human, Sir, Skeleton |
| exanima05 | Ancient, Human, Skeleton |
| exanima06 | Ancient, Golem Boss, Golem Guard, Skeleton |
| exanima07 | Fael 5, Ghoul 3, Ghoul H 35, Skeleton 1 |
| exanima08 | Ancient, Golem Suit, Human, Skeleton |
| exanima67 | Ancient, Ghoul H, Golem BAF, Golem BAM, HrtA Harvester, Skeleton |

## Open questions

- How the original builds the demon's nine extra parts (no template
  chunk covers bones 19 to 27); unreachable in the shipped game.
