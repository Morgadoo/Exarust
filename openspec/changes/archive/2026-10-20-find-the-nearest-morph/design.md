# Design

## Behaviour

The scan starts with index −1 and the maximum float. For each point it transforms the stored point by the owner's frame and measures the distance. A distance strictly below the best replaces the best, the transformed point and the index. An unordered distance is not below.

The model takes the already transformed points, so the transform is an input step.

## Model

`nearest_morph_index(point, points)` returns the index and best distance; an empty list gives −1 and `f32::MAX`.

## Open questions

The transform and distance bodies are not in.

## Docs

This method is 192 bytes. `RF_Morphing` moves from 1% to about 2%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
