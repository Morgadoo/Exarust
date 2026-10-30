# Design

Format pages: `docs/formats/OTHER.md` (Actors, which now states the item
routing; Underwear) and `docs/formats/RFC.md` (character records).

## Item routing

The library reads the database after each record once (`actor_items`) and
keeps it beside the record; `ActorCache::own_items` hands it out. The
viewer builds the duellist's look with `exa_world::record_look` over
`template_item`, the same routing a level's placed characters use with the
level's local database, so the garments carry their own appearance, hidden
slots and armour. Without a template the fixed debug outfit stays.

## Cull check

No code change. The probe that compared the culled triangles per merge
step became tests: with the braies alone the body keeps all eighteen slots
and the braies' 416 triangles take no cull bits; under a shirt the
trousers' pelvis node takes `0x200000` and loses 126 of 310 triangles, the
legs take `0x400000` and lose none.

## Open questions

- Whether the original shows the trousers' waist cut under a shirt exactly
  so: answered by the REA reading of the append (it drops a triangle whose
  mask word meets the bits); a retail capture of opponent01 would confirm
  the look.
