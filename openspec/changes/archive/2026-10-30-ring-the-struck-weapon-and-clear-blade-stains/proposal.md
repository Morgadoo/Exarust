## Why

The damage routine plays about ten sounds, one set per branch. Exarust rang the striking weapon on a clash and on a block and the shield on a block, but never the struck weapon on a clash. On a wound it played the weapon's set and the struck part's material at full volume, from its own rule instead of the routine's. The weapons' blood list was a tested model only: each weapon kept its blood on itself, and nothing cleared it when the original's world reset runs.

Phase 4 (combat and items) and Phase 5 (visual and audio fidelity) in docs/PLAN.md.

## What Changes

- On a clash's first ring of a swing, the struck weapon rings too. Below its record's split along it, it plays its record's class set (the haft sets). Above the split, or on a NaN projection, it plays its own sound. It plays at the blow's volume (no share) and 1.2 of its pitch, and its sound clock is stamped whether or not it rang.
- A wound (a turned one too) rings the striking weapon on every contact at volume × share × the striker's scale × the armour's muffling and pitch × the muffling: its haft's class set when the haft took it, else its own sound, and none without one. It stamps the weapon's sound clock. Each armour layer above the epsilon rings its class's set from the routine's own seven-set table (`FlsCol`, `HitClt`, `HitLtr`, `HitChn`, `HitPlt`, `HitHbr`, `HitGlm`) at 1.5 × the volume × the layer's weight × its class's loudness, pitch 0.4·pitch + 0.7. The audio side no longer plays its own full-volume weapon and part sounds on a limb hit.
- The weapons' blood list is a live resource. A weapon's first bleeding wound registers it (its tip length, clean spot), each wound adds half its R to its spot, and the weapon mirrors its spot's amount for the stain. A weapon that leaves the world releases its spot.
- The original's world reset (a new session, a save load, an arena round's start or end, a scripted jump, a quit) drops the list. The bodies it then rebuilds hold no blood. Exarust keeps the player's body and possessions across a level change, so the reset also cleans every surviving weapon, and the stain pass of a weapon whose blood no longer shows goes. A door's level change keeps the list, the weapon and its stain, as the original does.
- `--level-change reset:<level>@<frame>` changes level behind a world reset.

## Evidence

REA/Ghidra reading of Exanima.exe, assembly checked for the floats:
- the damage routine's clash, block and flesh branches;
- the struck-item voice: the clock stamp first, the projection summed in the routine's order, and the compare's NaN path to the item's own sound;
- the character class's set loads (the layer table and the haft sets);
- the world reset and its eight session callers;
- the door path, which keeps the player, its weapon and the list;
- the item part's slot release, which runs only when its node is destroyed.

## Out of scope

- The spray reset still runs on every teardown request. The original's door path does not reach it, but how the spray node survives that path's scene clear is not traced.
- The hidden-source bit (instanced objects): Exarust has none.
- A fist's sound clock: Exarust's fists have no item body to stamp.
