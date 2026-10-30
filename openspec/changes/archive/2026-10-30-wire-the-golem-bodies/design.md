# Design

## Bodies

The body generator builds a ragdoll from a model's bone nodes and the
shared skeleton's per-bone links, pins and spheres. The golem models carry
the human's eighteen bone numbers (`0100Pelvis` … `1800LHand`), so the
golem races join the human frame in `physical_frame`. Apparel stays with
the human frame. The race's stature, the body stats and the character
init's muscles (`character_muscles`, already used by the scene bodies)
give the golem its scale, arm relaxation and link smoothing.

## Sentry rest

The original keeps the statue-rest byte and the pose word on the body and
sets them each sentry tick. Exarust keeps one component (`StatueRest`) on
the body model, set by the sentry tick from the brain's statue byte and
the clip the body plays (the clip player starts a request at once, so the
current and requested motions are equal); the posture stage ORs it into
the drive's dormant flag (the sleep countdown by one a tick, the dormant
retention share, the 20 cm lift lead). The wake clears the component at
once, since the original's wake writes the pose word and byte directly.

The latch is state-flag bit `0x400`, which Exarust already knew as
"blows on it are turned". Held bodies are the parts carrying the weapon
bit (an unarmed body's fists, as the original's held list holds the two
hands).

## Open questions

- The haft blows' zero impact (see the proposal): answer with a trace of
  the original's reach routine for a scaled body holding a polearm.
- Whether the arm factor rewrites the held hands' contact mobility after
  a wake (Exarust's arm-word refresh does): left as the refresh does it.

- A resting statue's body slumps forward in its first seconds (the
  dormant drive's five link iterations against gravity on a 1.2-scale
  body) before it sleeps; with the rest disabled it stands upright. Answer
  with a capture of a dormant golem in the running original.

Formats page: none changes (`docs/formats/` describes no new field).
