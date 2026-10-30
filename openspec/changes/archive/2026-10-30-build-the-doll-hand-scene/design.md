# Design

The new rules stay decisions over plain inputs, like the rest of
`exa_items::doll` and `exa_ui::character_view`: the original's jobs act on
scene nodes and body parts Exarust does not model one to one, so each job
returns its steps (`ApplyStep`, `ModelRelease`, `HandNode`) and the caller
performs them.

- The rebuild job copies the record's other-set item ids into its own
  record; its body refresh tests those copies, so `AttachPlan::targets`
  carries them and `refresh_body` compares them with 0.
- The call made after every attach recomputes the body's weapon class word
  and hold kind from the parts it holds; it is `RefreshGrip`, distinct from
  the stance restart (blend 20) that the swap state skips.
- The take-out's model finaliser is a nested routine reading the take-out's
  frame: its restart test reads the job's mode word, so `take_out` keeps
  `job.mode & RESTART_MODE`.
- The weapon and light attach jobs store `flags & 0x8000_0000` as the record
  entry; the model's flip and lit marks still come from the full flags.
- The figure's drag-start sets the drag kind on the drag record and builds
  a separate payload (icon, item flags, half icon size); the plan reuses
  `drag_start` for the kind and `drag_over_cost` for the cost.
