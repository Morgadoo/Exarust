# Design

Format pages: none (runtime behaviour); the dialogue notes live in
`exa_script` and `exa_game` crate docs.

## Decisions

- One store, two copies. The original keeps one contact record that the AI
  and the scripts both write. Exarust keeps it in the brain and mirrors it
  into the cast; a cast value that differs from what the mirror last wrote
  is a script's or a line's write and goes back to the brain before the
  next mirror. Writes are compared bitwise so an unordered value is still a
  change.
- When. The mirror runs in PreUpdate (before the fixed step, so the AI
  ticks on the scripts' writes) and in Update after the cast mirror (so
  the frame's scripts read what the AI decided). The greeting, which runs
  inside the brains' chain, syncs its brain itself so the mood sees the
  level the pass just set.
- Trust. The mood's and `Trust`'s attitude is the trust query: the
  relation's last attitude with the mind powers' overrides. The brains run
  no mind powers, so the mirrored attitude is the relation's last value
  (0 with no relation, which is what the original's query answers after it
  creates one).
- Arousal. The personality's arousal (10) and the integration's readiness
  are one word in the original. The brain now starts its readiness at the
  personality's arousal, and the cast reads the readiness.

## Open questions and how they were answered

- Which contact fields the mood reads: REA on the mood routine — level
  (bit 2 hostile, 1 watched), concern, hostility; attitude from the trust
  query. Answered.
- Which fields the speech hearing writes: REA on the hearing routine —
  noticed level and attention offset of the listener's contact for the
  speaker. Answered.
- Whose contacts a broadcast walks: the actor's own contact list. Answered.
- Where `WitnessRole` measures from: the operative object running the
  script. Its only shipped caller is the Pedestal Hand's use script
  (corpus search of the packs); object use scripts do not run yet, left as
  a stand-in.
- What the player behaviour writes in its contact records besides the
  other character: not traced (labelled guess: nothing the mood reads).
