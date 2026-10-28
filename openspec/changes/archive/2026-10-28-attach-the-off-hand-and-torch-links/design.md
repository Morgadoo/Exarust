# Design

- The off-hand pass runs after the dresser and reuses `held_item_bodies`
  and the grip stage; nothing new runs per frame beyond the grip stage's
  existing work and three small systems (death weld, collapse release,
  pending grips) that skip released items.
- The shield's links are computed from the live transforms: the item in its
  forearm's frame, and that point less the left hand's rest translation for
  the face pivots, as the original does without a turn. Exarust parents the
  shield node under the hand bone turned −5°, a guess at the forearm frame.
- The flip progress lives per held item; only the first item turns, so one
  weapon behaves as the original's per-character progress.
- The limit law is kept as a tested function for root world links: the
  registration gives every parented skeleton link the pivot flag, so the
  stored RSQ words (all 0) are not carried into a character body.
- The equip refresh already exists as `weapon_record::equip` (the race
  index byte is the 0x500/0x600/0x700 word); no second refresh is run.
