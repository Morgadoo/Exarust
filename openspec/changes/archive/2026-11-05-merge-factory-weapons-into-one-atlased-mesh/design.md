# Design

Extends the item-instances spec (`openspec/specs/item-instances/spec.md`)
and builds on mesh-editing's editable part assembly, regroup, weld and render
form, which were ported but had no caller. The format page is
`docs/formats/OTHER.md` (Factories, Assembly and Paint).

## The merged mesh

- An empty editable mesh (position channel and stream only) receives every
  part. Its frame is the identity, so a part's positions are placed by the
  part's own frame.
- The part frame: the turn of (x, y, z) angles built with the runtime
  library's sine and cosine (the x87 `FSINCOS`), each of its three rows then
  multiplied component-wise by the part's scale. The scale therefore acts
  along the weapon's axes after the turn, not along the part's own. The
  position row is the placed position.
- The UV map is `uv · size + offset`, a multiply then an add in single
  precision, on the first stream whose low key is the texture coordinates.
- The groups' split flags come from the components' materials: the render
  form runs before the weapon's material is assigned. Shipped components
  share one material name, so a weapon has one group.
- The bounds refit after the render form gives the striking extents'
  denominator (`centre.y + half.y`).

## The atlas

- The region's draw is unchanged (target size, tiles, crop, UVScale,
  emblems). It now blends into the atlas texels its quad covers: the first
  covered texel along an axis is the one whose centre is at or past the
  quad's edge; a negative size runs the region backwards from its offset.
- Pass order: every region's colour and normal passes, then every region's
  specular pass reading the finished normal target. The targets hold bytes
  between regions.
- A region without the component's normal map leaves the normal target as
  it is (as before, the normal pass needs the item normal).

## Open questions and how they were answered

- Does a deform refit the part's box? No: the five deform routines only
  move the position stream (REA). The striking extents keep the stored box.
- Do the chosen parts' regions overlap? Yes, in 64 of the shipped recipes
  with zero seeds (`shipped_factory_weapons_assemble` lists them). Hence the
  shared-target blending rather than per-region textures.
- Is the merged top the denominator? The save check
  (`saved_factory_weapons_keep_their_striking_extents`, a real save from
  `EXA_FACTORY_SAVE`): 110 of 143 weapons match the saved start and end
  bytes with the merged top, 6 with the highest placed box top.
