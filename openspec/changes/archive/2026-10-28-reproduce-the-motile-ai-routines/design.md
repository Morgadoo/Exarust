# Design

- **Room cells.** Exarust keeps a vector per 600 cm cell (`rooms::RoomCells`)
  instead of the original's intrusive list per level room. No room table is
  loaded, so each in-grid cell stands for a room. A position off the grid
  changes nothing, as in the original. The nearby gather scans the
  fighters in the brain's cell. That is O(brains × fighters) a frame, which
  is cheap at a level's character count. The list is kept on the brain,
  but nothing reads it yet.
- **Idle turn.** The original turns the body through its locomotion toward
  the AI's eased yaw. Exarust's NPCs have no locomotor, so the AI tick writes
  the eased yaw (not the heading, which is the bare look) to the model's
  rotation while the brain is not engaged. An engaged duellist's combat
  tick seeds its heading from that rotation.
- **Level-leave reset.** The original calls this virtual on every character
  from the level export, so it is not part of presence. It stays a
  function until Exarust writes a level on leaving.
- **Ally sense.** The original's contact words are mapped to Exarust's
  `Contact` as the standard sense maps them: noticed, noticeability,
  concern, concern target, alarm, threat, closeness, view, selected. The
  word holding 100 for a fellow AI is taken to be the attitude (the
  standard sense writes 1 there for its special case). Its alarm branch
  needs the observer to be a statue, but a statue returns before reaching
  it, so the branch is left out.
