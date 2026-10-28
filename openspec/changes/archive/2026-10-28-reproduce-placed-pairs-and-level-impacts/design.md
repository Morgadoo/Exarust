# Design

- The placed state is not a separate flag: the original's integration
  clears the placed bit as soon as the settling countdown is spent, so
  `Ragdoll::placed` reads `settling() > 0`.
- The placed pass clamps its weights at zero for both the push and the
  offset record; the placed callback normalises its weights to sum one and
  records the offset with them unclamped, then pushes (B placed: offset
  first, A placed: push first, as the original orders them).
- The level impact groups a part's accepted hits over its detection call;
  the live level is one triangle soup, so a call is one part against the
  whole level. The mean point is the sum times the count's reciprocal.
- `sector::registration`, `impact_scale`, `static_mask`, `body_mask`,
  `reset_overflow` and `BroadPhase::for_scene` are tested functions: the
  loader produces no runtime node or scene flag words to feed them.
- The level impacts are recorded on the body but nothing consumes them yet.
