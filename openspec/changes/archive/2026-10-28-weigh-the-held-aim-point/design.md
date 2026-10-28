# Design

- **Aim weights.** The original's barycentric routine takes four triple
  products and multiplies each by one exact reciprocal of the determinant.
  `exa_physics::engine_math::barycentric_exact` already does this in the
  same order, so the refresh uses it. Exarust keeps a part's rest points
  in metres, while the original keeps centimetres. The refresh scales the
  points to centimetres first, so the weights match up to rounding. A flat
  rest tetrahedron gives no weights instead of infinities. The original's
  rigid-body registration never makes a flat one.
- **Reach.** The refresh writes its held reach, and the equip pass then
  zeroes the same word and takes the largest record reach. Unarmed, it
  takes the race's reach. Nothing reads the refresh's value, so no reader
  is wired. `exa_ai` keeps publishing the record reach.
- **Swap.** The original's switch re-equips the weapons, and every equip
  routine runs the moveset choice on held slot 0. Exarust's choice reads
  `Hands.main`, so the swap trades `Hands.main` and `Hands.second`, and
  the existing change-driven system picks again. Two one-handers keep the
  two-weapon moveset. Only a pair of different classes changes moveset.
  The location of the original's swap code is a guess; the routine it
  ends in is not.
- **Hand words.** Exarust's hand parts have no separate link-mobility or
  mass word, so the unarmed share stays a tested rule beside
  `empty_hand_words`.
