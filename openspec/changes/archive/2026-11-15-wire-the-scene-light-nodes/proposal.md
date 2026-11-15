## Why

Roadmap tasks B1 and B4 (wiring lane `scenenodes`, docs/roadmap/phase-b-close.md).
The live scene of a tile-grid level registered meshes only, so its light
arms never ran: no light node, no alias, the membership rebuild's alias half
and the per-alias buckets always empty, and the room light lists kept by a
separate model (`rooms::RoomLights`) instead of the scene's aliases. The
alias weight sort was also modelled on a light node, while the original
sorts a mesh's bucket list.

## Evidence

REA (Exanima.exe), checked against the assembly where order or compares
matter:

- The activation pass (scene unit) makes a moved light's alias in its
  sector, stores its cell and, in a grid scene, floods an omni light (kind
  3) or fills the cells for any other kind. Outside a grid it spreads the
  alias along the linked sectors instead.
- The flood calls the room manager's cell hook on every insert (a new
  entry under twelve, or a widened one). The hook (installed in room mode,
  which the game scene always sets) searches the light's alias list for
  the alias owned by the cell's room sector and stamps it with the frame
  counter, or makes one there (stamped, moved, the sector marked
  light-added). The flood then removes every alias of the light whose stamp
  is not the frame counter, walking the list by index up to its starting
  length while the removal swaps the last alias into the freed slot.
- The alias constructor appends the alias to its light's list and does not
  stamp it.
- The weight sort reads the list head and the 24-byte records of the
  node's bucket list (the same fields the bucket rebuild writes), so it
  orders a mesh's buckets; the node passes call it after the alias link
  tests (a moved mesh always, another when a test lit it). The compare is
  COMISS with JP/JC: a NaN stops the walk.
- The find-or-make alias routine and the spread are reached only from the
  batch insertion and the activation of a scene without a grid and from
  the directional fill: a grid level's lights are omni and get their
  aliases from the cell hook.
- The published sun, reach and sky records are written by the scene
  environment's apply; only the rain effect reads the sky record directly.

## What changes

- `scene_nodes`: a light's alias list holds the sectors of its aliases;
  `flood_hook`, `drop_unflooded`, `alias_in`, `activate_lights_with`; the
  frame counts frames and takes the alias link test (`AliasLink`) and the
  flood; `order_buckets` replaces the light-node link list.
- `scene_live`: point lights are light nodes of their room's sector,
  refreshed when they move and moved when their room changes; the frame's
  flood answers the rooms of the cells the light flood recorded.
- `shadow_links`: the flood runs before the scene frame; the link pass's
  sector test reads the scene's aliases.
