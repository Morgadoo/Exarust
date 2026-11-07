# Build every race's body

## Why

Exarust built a physical body only for the human frame (human, skeleton,
ancient), the golems and the wraith. Every other race the shipped game
places got a brain but no body: the 38 ghouls and 5 Fael of exanima07,
Sir in exanima04 and the harvesters and hulking ghouls of exanima67 could
not stand on their feet, walk, fight, be hit, collapse or die. The ghouls'
halved chest pins (round 99) never took effect.

## What changes

- `exa_game::scene_bodies::physical_frame` is true for every race byte but
  the demon's (11). The original builds every character with one builder:
  the race model's parts, each bone taking the one template body that
  ships (`humanoid.rsq`), then the character init, whose race-class
  branches only re-tune that body. Every race model but the demon's
  carries the template's eighteen bone numbers (the wraith its twelve
  upper ones).
- The level's placed characters of those races now get the simulated body,
  the character init's muscles (the ghouls' halved chest pins among them),
  a mind and a duellist, as the human frame does. Their dead are built as
  placed corpses. Only the human frame is dressed, as before.
- The load log counts the bodies built per race and how many are physical.

## Not changed

- The demon (byte 11): its model has 27 bones, nine the template does not
  cover, and no shipped level or arena spawn builds one; it stays without
  a body rather than guess its extra parts' links.
- The scene bodies (arena fighters, the Beast) already took a physical
  body for every race.
