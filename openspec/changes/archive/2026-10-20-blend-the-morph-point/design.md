# Design

## Behaviour

The block's base point at `+8` is the starting result. Each of the `0x4c` entries at `0x50`, stride 32, carries a point from the block and a weight at offset 0. A weight whose magnitude reaches the epsilon adds the point times the weight to the result. The result is copied to the output.

## Model

`blend_morph_point(base, points, weights)` returns the weighted sum, skipping weights under the epsilon.

## Open questions

The accumulate call is not in.

## Docs

This method is 268 bytes. `RF_Morphing` moves from 7% to about 8%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
