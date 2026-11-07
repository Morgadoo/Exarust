# Build the demon's body, give unarmed races their reach, skip absent skins

## Why

- The demon (race byte 11, `demon a01`, brought in by the summoning that
  exanima67's GdnShaperD knows) had no body: its model's nine extra nodes
  (a seven-part tail and two wings) have no skeleton bone.
- The duellist read every unarmed fighter's reach as a human's 50 cm, so
  a clawed race's reach (the ghouls 1 m, the harvester 1.5 m) never
  reached the AI.
- Every race's character asked for `<stem>difm`, `spcm` and `nrmm` skin
  maps, which ship only for the human and the ancient.

## What changes

- Every race is physical. A model node with its own body chunk and no
  skeleton bone becomes a plain rigid body hung from its parent part: its
  spheres, its links with their strength and smoothing (soft welds where
  flagged), links that are never muscles (the init's tone of 0.01), and
  its registered mobility over the character init's limb mass factor.
- `CharacterStats::unarmed_reach` carries the race's unarmed reach; the
  duellist's reach and the hit reaction's threat use it when unarmed.
- `SkinLook::textures`: only a race whose look word has a texture bit
  re-skins its body; other races keep their model's materials.
- `--spawn-race n@x,y,z` (debug) places a living character of a race.
