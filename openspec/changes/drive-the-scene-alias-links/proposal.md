## Why

Roadmap task B4 (wiring lane `scenenodes2`, docs/roadmap/phase-b-close.md).
Round 229 made the live scene's lights and aliases real, but its node
passes still ran a stand-in link test (weight 0, no faces): the alias link
methods the original installs on every alias had no live caller, the
alias sync after a light's activation was only reported, and a mesh's
light buckets kept their alias order.

## Evidence

REA (Exanima.exe), read in the assembly:

- The alias constructor installs the gated link method for a light whose
  option word carries `0x40000` and the plain one otherwise. That bit is
  the level record's ranged flag (`exa_formats::rfc::RANGED`); the model
  had read it as a shadow cube.
- The gated method asks the cell flood's wall test when the option word
  of the alias's own sector carries `0x2000000`: the grid setup sets it on
  the scene and the room manager on every room sector it makes (not a
  light flag, as the ledger read it).
- It writes the alias's shadow map word only past its intensity gate, and
  only for a node with the casting bit; a miss leaves it. The plain method
  sets the word to 1 for a lit node; its unlit answer is the low byte of a
  stack address (the luma call returns its argument pointer).
- The node colour the methods multiply by is the geometry activation's
  bound: per channel the largest of its parts' materials' specular times
  level plus diffuse, the sum the material activation stores after its
  record loader linearised the colours.
- The activation runs the alias sync over the light's alias list after the
  flood: each alias takes the light's direct colour, luma, falloff,
  radius and frame, the light's node under its sector (the sector-child
  walk), and its moved word is set. The flicker step copies only the
  render colours to the aliases.
- The membership rebuild's sector callback (`+0x340`) is stored by no
  instruction on a sector (the only stores at that offset are the grid
  builder's own records), so the branch never runs.

## What changes

- `light_alias`: `reach_with` (gate asked last, same answer), hits report
  whether they reached the face mask.
- `scene_nodes`: `alias_reach` (the two methods and their shadow word
  writes), `sync_alias_links` in the activation, `LIGHT_RANGED`,
  `HAS_COLOUR`, `SceneNode::{link_light, colour}`, `LightAlias::{link,
  anchor}`; the link closure takes the scene.
- `scene_live`: light nodes keep their link values, meshes their colour
  bound from the material, and the frame runs the real methods with
  `LevelWalls::sees` as the gate; `--no-alias-links` to compare; the world
  sector deletion warning is logged.
