# Design

- `lift_point` tests the climb first because the original refuses a
  climbing ray after either branch; the kept-hit test is `limit < height`
  so a NaN height takes the plane. `place_point` keeps the constrained drop
  helper's `height < limit` fallback and its climb test on the plane branch
  only.
- The free placement pulls the grab point by a spring, so its placed frame
  is not the part's; the part's frame as it stands when the placement holds
  is kept instead, and a marker snap keeps the marker's frame (the part's).
- The repair's copies reuse the refresh's tails: `refresh_weapon` and
  `refresh_apparel` now end in `weapon_condition_pass` and
  `apparel_condition_pass`, the same routines the original calls from both
  paths. The model price word is passed as `Option<&mut u16>` plus the
  template's, mirroring the original's skip when the copy has no model
  section and its return when only the template lacks one.
- `support_corrections` sums each axis dot x, y, then z, as the engine's
  row-dot helper does.
- The GUI drop's database lookup is gated on an untagged id, since
  `DollItems` masks ids.
