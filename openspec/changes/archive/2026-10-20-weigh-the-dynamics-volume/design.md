# Design

## Behaviour

The grid is padded by one cell on each side, so its strides are `(dims + 2)` and the walk runs from 0 to each `dims` inclusive. A filled cell (`count > 0`) adds `i − 0.5`, `j − 0.5` and `k − 0.5` to the centre sums and one to the filled count. The centre is `(sum/count/dim − 0.5)·scale`; the mass is the filled count times the cell volume `(2·scale.x/dim.x)·(2·scale.y/dim.y)·(2·scale.z/dim.z)`. With no filled cell the centre is zero and the mass is 0.1.

The sort is the original's two-way quicksort: pivot from the middle, advance the left while the key is below it, retreat the right while the key is above it, swap while the indices have not crossed, then sort both sides. Unordered (NaN) comparisons stop both scans, as the original's `jp` does.

## Model

`summarise(dims, scale, counts) -> VolumeSummary` and `sort_by_key(entries)`.

## Open questions

The grid construction, solid test, corner masks and sphere extraction are separate.

## Docs

`RF_Dynamics_Volumes` is **5%**. The headline rises slightly.
