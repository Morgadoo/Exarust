# Tasks

- [x] Rebase the lane on main (already current).
- [x] Check every rule the lane added or changed against the original's
  instructions; correct the standing IK's targets, snap and lift input,
  the locomotion IK's anchor/contact reads, sign and NaN paths, the jolt's
  threshold, latch, step end and notice, the footstep draws, the travel
  phase's root, the release race's draw, and the muscles-off height ease
  and NaN plant.
- [x] Keep the sole floor on the Ground Query's miss branch and drop the
  extra level query.
- [x] Remove the stand-height block, the duplicate queued-step entry and
  the footwork-velocity queueing.
- [x] Remove raw object offsets from comments and docs.
- [x] Update docs/formats/RSQ.md and the crate READMEs.
- [x] Tests and clippy for exa_physics, exa_game, exa_combat, exa_anim,
  exa_audio and exa_formats; a GPU check of standing, walking and a duel.
