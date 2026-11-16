## 1. Read the original

- [x] 1.1 The transfer's skin branch and its record merge, in the assembly
- [x] 1.2 The race loader's record build, the merged-mesh walk and the nearest copy
- [x] 1.3 The garment slot walk's order of transfer and skin build
- [x] 1.4 The group stack's push, empty push, pop and reader

## 2. Port

- [x] 2.1 The skin branch in `MorphBody::transfer` (`PartMorph::skin`)
- [x] 2.2 `body_skin`: merge, weld and nearest records; attach to the morph body
- [x] 2.3 The model build keeps the transferred weights; skin-only garment variant
- [x] 2.4 `GroupStack` as the original's frames, driven by the skin pass

## 3. Check

- [x] 3.1 Unit tests of the merge, normalisation, nearest search, transfer and relaxation
- [x] 3.2 Shipped data: the records, the garments' weights against the seam weights, the stack's reach
- [x] 3.3 fmt, clippy, workspace tests, openspec validate, play build
