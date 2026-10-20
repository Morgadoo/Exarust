# Design

## Behaviour

`needs_kaiser_mips(mip_count, width, height)` is true when the stored count is at most one, both sides are powers of two and both are at least 4. In that case the decoded RGBA8 base goes through `build_mip_chain_rgba8`, which normalises each byte by 255, builds the f32 chain (wrapping taps), quantises each level back and returns the base followed by the levels plus the total count. The image's `mip_level_count` is set to that count. Other images keep their stored levels unchanged.

## Model

`build_mip_chain_rgba8(base, width, height, wrap) -> (Vec<u8>, u32)`; `needs_kaiser_mips`.

## Open questions

The NPOT fallback and the original's GL upload are separate.

## Docs

`RFP_Mipmaps_Kaiser` is **40%**. The headline rises slightly.
