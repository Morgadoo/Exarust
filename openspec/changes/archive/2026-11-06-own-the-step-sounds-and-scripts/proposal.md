## Why

Stage 6, 1a part 3, second half (first part): the sound bank, the audio environment, the clips, the conversations and their contact mirror were still lent to the step because fixed-step systems played sounds and ran scripts.

## What Changes

- `exa_audio::step_sounds`: plays by set or clip name on the frame (sets loaded on the frame); the fall, shock, kinesis and throw sounds are handed to the frame with `FrameCommands::send`; voice-line availability reads the sound pack (`SoundBank::set_size`).
- Throws: the lift's voice is kept by the frame under a key the step hands out; `ThrowFade` carries the key.
- The alarm's wake pitch draws from the step's own engine generator (`AlarmRoll`), as the original's process-wide generator is drawn from by both threads in no fixed order.
- Encounters and greetings: REA shows the original calls the roles unit's encounter and the greeting from the AI's target pass (simulation thread) while the dialogue runs the same engine on the main thread, with no lock. The step takes the pass reports and the relation query; the frame runs the scripts after the step with the session cast (`encounter_on_frame`, `greet_on_frame`); the locale crosses as `ScriptLocale`.
- SoundBank, AudioEnvironment, Assets<AudioSource>, Conversations and ContactMirror are no longer bridged.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Rule change: the scripts of an encounter or greeting run after the step that reported them (before: within it); the alarm's pitch draws no longer interleave with the frame's sound draws.

## Out of scope

- The collision world (Physics, StandIns, ColliderNodes) and the navigation grid; the free loop.
