# Proposal

## Why

The original's prop reader replaces the object node's static bit and its
two visibility-rule bits with the placement's (`flags & 0xB80000`), and
the importer's finalise pass applies the rules to the roots of the load's
node list: `0x100000` hides every light in the tree, `0x200000` hides
lights and fires once the level's daylight scale is above 0.12. No
shipped model node carries the bits, so in 0.9.5 they come only from prop
placements: arenao1's three lanterns and arenao3's three braziers (and an
exanima03 bed with no light). Exarust read the rules only from the model
file, so those lights always burned.

Evidence (REA, `Exanima.exe`): the prop reader's flag merge, the finalise
pass's root test (parent index below 1 or past the count) and its two
walks; a corpus test over every shipped level.

## What Changes

- `exa_formats::level::Prop::NODE_FLAGS` (`0xB80000`).
- `exa_world`: `with_placement_flags` writes them over the prop's object
  node before it spawns; `node_hidden` reads only the root's rule.
- `Scene::apply_visibility_flags` reads roots only.
- The rules' parameter is the daylight scale it carries (`daylight`,
  `DAYLIGHT_LIMIT`), not "darkness".

## Impact

`exa_formats`, `exa_world`; docs/formats/RFC.md, the crate READMEs.
