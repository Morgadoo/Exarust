# Design

`solve_among` asks every stage for the coupled bodies before the link
iterations and again before the friction passes (the pairs found this
tick). `each_body(bodies, select, f)` collects the selected bodies and runs
`f` on them with `par_splat_map_mut` on the compute pool (inline under two
bodies). A body's `step`, `iterate`, `detect`, `rub` and `post_step` read
and write only that body (and the shared, read-only level), so their order
across bodies does not change any result. `Pairs::coupled` marks the bodies
whose key (first part's entity) appears in a kept pair; the pair indices
are resolved later, by the same keys.

The level type must now be `Sync`; rapier's world and the test levels are.
