# Design

## Behaviour

The blob starts with the magic and size, then the base (12 bytes), the rest points (`V·12`), the part count, and each part's pivot (12) and deltas (`V·12`). Restoring checks the magic, copies the rest points over the owner's positions, writes the base to the owner and refreshes the owner.

The weighted base starts at the blob's base and, for each part, adds `pivot · weight` when `|weight| ≥ 1.19209e-7`. The full evaluation does the same to the base, then for every vertex adds `delta · weight` to the rest point, writes the base and refreshes the owner.

## Model

`restore_morph_positions(data)` returns `(base, rest)`. `morph_weighted_base(data, weights)` returns the base. `evaluate_morph_positions(data, weights)` returns `(base, points)`.

## Open questions

The owner refresh is outside the data model.

## Docs

`RF_Morphing` is **51%** (8,755 of 17,099 bytes). The headline stays 14.4% and 5.7%.
