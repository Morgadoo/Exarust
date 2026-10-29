# Proposal

## Why

The motile body (TMotileDynamics, 54 %) still wrote no planted-foot flag
into its captures, so the muscles-off sampler read the source key's flag
where the original reads the capture's, planted feet of bodies that were
not walking by the keys' flags and never lifted a foot. The swing voice's
lifecycle (start, rustle, per-tick fade) and the walking plant's clothing
rustle were also missing.

## What Changes

- `exa_anim`: `Animator::note_planted` / `captured_planted` (every capture
  notes whether each foot touched); `take_capture` reports whether it
  captured; `Animator::ragdoll_lead` (the muscles-off sampler leads in
  from the capture while it aims at the capture's key, without ease, and
  keeps the clock when the lead ends); `sample_ragdoll` returns its
  bracket.
- `exa_physics::body`: `limp_foot` / `LimpFoot` (the muscles-off feet
  rule), `limp_anchor_height`, `limp_step`, `walk_rustle`;
  `Ragdoll::limp_walking` / `limp_supported` (kept when the muscles go).
- `exa_game::limp`: the clip clock notes the feet after each capture
  (data only); `sample_limp` reads the capture's flag and applies
  `limp_foot` — tested, not scheduled.
- `exa_combat::swing_voice`: `SwingVoice::enter`, `voice_speed2`,
  `swing_rustle`, `fade_voice`; `SwingState::in_region` — tested.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the captured planted feet, the muscles-off feet rule,
  the swing voice, the walking rustle.

## Impact

- `exa_anim`, `exa_physics`, `exa_game`, `exa_combat` and their READMEs.
- Evidence: the full disassembly of the three captures, the muscles-off
  sampler's lead-in and feet block, the swing routine's voice start and
  update, the body tick's held-voice fade, the living sampler's walking
  rustle, and the foley set-up (which fields hold the volume and base),
  read as instructions; REA/Ghidra pseudo-code for structure.
- No live behaviour changes: the clip clock only stores the flags.
- Out of scope: playing the swing voice and the rustles through
  `exa_audio` (the live whoosh stays the window-open one-shot), the foot's
  water splash, scheduling the muscles-off sampler.

## Phase

Phase 3, bodies: skeleton, physics, control (TMotileDynamics kernel lane).
