# Design

## Behaviour

Starting at level 1, `dst = base >> level`. Generation stops when either dimension is zero, their sum is below 3, or a dimension does not divide the base exactly. Otherwise the level's Kaiser table is applied horizontally to every row of the base, then vertically to every column of that result, producing an interleaved RGBA level. Each level reads the base, not the previous level.

## Model

`build_mip_chain(base, width, height, wrap, max_levels) -> Vec<MipLevel>`.

## Open questions

The NPOT fallback and upload are separate.

## Docs

`RFP_Mipmaps_Kaiser` is **30%**. The headline rises slightly.
