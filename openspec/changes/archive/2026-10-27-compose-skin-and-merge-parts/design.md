# Design

The composite is a CPU function over `paint::Texels`, sampled bilinearly
with wrap, rounding each target half to even. The beard's mip-3 alpha is an
8×8 box average and its plain read is taken at the base level; both are
labelled guesses (the mip chain is not in these units).

The painted stem is the only change on the running path: `outfit_painted`
schedules one paint job per garment and `equip_outfits` gives every `&`
part the painted material of that stem. Two shipped garments with two
stems now show their first stem's textures on both parts, as the original
does. Load time falls (one job per garment instead of one per stem).

The merge keeps the original's order (append order, materials first seen,
each run's vertices in appended order) in a single flattened `Mesh`; the
original's separate per-attribute streams are not modelled.

The parent context keeps `auto_skin`'s behaviour unchanged when no parents
are passed, which is how the model build calls it.
