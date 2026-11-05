# Proposal

## Why

This is Phase 4 (combat and items) of docs/PLAN.md: the weapon factory
(FLX_ItemFactory). Exarust built a factory weapon (an `@` item: the golems'
halberds, every assembled sword, axe and polearm) as one node per part, each
part painted into a texture set of its own. The original merges the parts
into one mesh and paints every part into one shared atlas that
the merged mesh's one material reads. The striking extents it writes into the
item are fractions of the merged mesh's top, which Exarust could not compute
without the merge.

Evidence (REA, Ghidra listing of `Exanima.exe`):
- The factory's assembly: for each chosen part in recipe order whose
  component loads, the component's first node (a mesh) is deformed, its matrix
  reset and rebuilt (the turn of its angles, every row scaled
  component-wise by the part's scale, its placed position), the first
  texture-coordinate stream mapped by `uv · size + offset` (the part's atlas
  rectangle), and the mesh appended to one new mesh named after the factory.
  After the last part: regroup by material, weld, render form, bounds refit,
  and the run-time material (`_procmat1`, or `_procmatr` when an area's
  material has an alpha program) assigned to every sub-object.
- The append (the mesh assembly routine) transforms only the position
  stream and skips the normal stream; the render form rebuilds the normals.
- The paint job: one colour, normal and surface target at the factory's
  atlas size (halved below texture detail 3), cleared to black, (½, ½) and
  (0, 0.01, 0); every region's colour passes, then every region's normal
  passes, then every region's specular passes. Shipped recipes overlap their
  regions on purpose (a sword's guard spans the blade's columns, a halberd's
  grip the whole atlas).
- The builder's striking extents divide by the merged mesh's box top (box
  centre plus half extent). The deform routines move the position stream and
  never refit a part's box, so a head's stored box can reach past the merged
  top: a real save stores end bytes of 201 to 206 for some tools.

## What Changes

- `exa_assets::weapon`: a factory weapon is one node with one mesh: the
  parts' component meshes, deformed, mapped into their atlas rectangles and
  placed at the original's part frame, appended into an empty editable mesh,
  regrouped, welded and converted to render form (normals rebuilt). Its box
  gives the striking extents' denominator. Components are cached with their
  stored box and split flags.
- `exa_assets::atlas` (new): the job's atlas size, the region placement
  (mirrored for a negative size) and the shared-target paint in the
  original's pass order.
- `exa_assets::paint`: the region painter split into its colour-and-normal
  and specular passes over a caller's targets (`RegionJob`, `Canvas`), so a
  region blends into the shared atlas.
- `exa_assets::model`: `factory_atlas_material` paints the regions into
  the atlas and makes the weapon's one material; `painted_factory` is gone.
- `exa-tool factory <name> <out.png>`: assembles a factory weapon and
  writes its atlas, the atlas with the merged mesh's texture coordinates
  drawn over it, and its surface map.

## Out of scope

- The alpha job's sRGB-alpha colour target (`_procmatr`): the atlas is
  opaque. The run-time material name is recorded on the node only.
- Texture-only overlay parts (part flag 1), the mask-gated shade of parts
  flagged 2 and the alternate normal program (part flag 4).
- A crop above 1 (a negative texture parameter, e.g. combknifea1): the
  original's quad overhangs the region into the atlas; Exarust draws the
  region only.
- The 33 of 143 saved factory weapons whose striking extents still differ
  (tools: AxeTool, Hammer_Sld, Sickle2H, BillTool2H, Mallet, PickAxe): the
  generation side, not the merge.
- TProcMaterial's GPU compile and the impact sound by class: unchanged.
