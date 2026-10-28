# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the thaumaturgy powers
(TForceBlastEffect, TForceTorrentEffect and their geometry and emissive
classes): the force blast and torrent simulated their chunks and streams
but drew nothing of their own, met no level surface, and a struck
character took only energy and a knockback step. The blastfx lane built
the ribbons, the contact walks, the release alert and the character hit;
its review checked every rule against the instructions and corrected the
ones that differed.

## What Changes

- The blast's and torrent's ribbon meshes (`effects::ribbon`,
  `Blast::ribbon`, `Torrent::ribbon`), drawn each frame with the glow
  stand-in (`thaum::blast_live::draw_blasts`,
  `thaum::torrent_live::draw_torrents`).
- The release alert to the caster's known characters within 18 m
  (`effects::blast::alert_amount`, gain 3) and the character hit's
  instability counter +1 and concern 1 at 0.01, for the blast and (once a
  tick, from the gathered impulse) the torrent.
- The level contact: thrown chunks and running points swept as balls of
  their static reach; a chunk rebounds (a bolt's sampled walk puts it on
  the contact, zeroes and wears it), a point moves half way.
- Review fixes from the instructions: the knock's squared length order
  and NaN pin gain, the kept weight's 0.9 and the torrent's character
  push in double precision, the blast quad's previous point (the previous
  call's point, giving inner quads the neighbours' central difference),
  the head's face pass sign and single-precision subtraction, the bolt's
  sampled walk only on static bodies (the limb walk is the face walk for
  every blast; the flag is the piercing flag), the torrent's limb reach
  of ten (forty is the simulated objects' point walk).

## Evidence

The force effects' character hits, alert, static dispatch, face and
sampled walks and their callbacks, the torrent's hit scan and point-cloud
walk, the quad emitters and the ribbon draw's face passes, read from
`Exanima.exe` (instructions, not the pseudo-code, for every sum order and
precision).

## Out of scope

The GL passes and effect shaders, the sounds, GI injection of the lights,
the support-pin gain and 5 s recoil consumers (Exarust's body has neither
field), the charging blob's own draw, the engine's exact face and point
queries (a rapier swept ball and Exarust's limb spheres stand in), the
simulated objects' points, and the player's turn spring.
