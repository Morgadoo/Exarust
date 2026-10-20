# Design

## Behaviour

The table is 256×256 words. The clear writes the blocked value to indices 0 through 255 (the first row), to `0xfeff` through `0xffff` (the last row plus the last entry of row 254), and to indices `row·256` and `row·256+255` for rows 1 through 254. That covers the whole border. A table shorter than 65,536 entries is left alone.

## Model

`PATH_STAMP_SIDE = 256`, `PATH_STAMP_BLOCKED = u32::MAX`, `clear_path_stamps(&mut [u32])`.

## Open questions

The constructor's global byte at `0x49d550` is not modelled.

## Docs

`TPathFinder` is **64%**. The headline stays 17.0%.
