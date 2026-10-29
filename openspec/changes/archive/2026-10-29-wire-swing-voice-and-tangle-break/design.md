## Context

The swing routine runs in `exa_combat` (`run_swings`, the character tick's attack stage), the mixer in `exa_audio`. The original writes the voice record's gain and pitch directly; Exarust's voices live in `AudioEnvironment`, which `exa_combat` does not see.

## Decisions

- **Cues, not shared state.** `run_swings` sends `SwingVoiceCue::Start` / `Sound` and a body-tick system sends `Fade`; `exa_audio::swing_voices` keeps one voice per fighter (allocated at its first swing, freed when the fighter is gone) and applies them: a start is the environment's set retrigger (the original's), the sound writes the voice's gain and pitch, the fade takes 0.01 and 0.0005 off a playing voice and sets its stop flag under 0.005.
- **Who has a voice.** Only a fighter holding an item: fists and claws are not held items in the original, so an unarmed swing is silent.
- **Loudness and gain are 1.** The constructor sets both to 1.0; only the force swing and scripted fades change them, which Exarust does not have.
- **The rustle** uses the mixer's generator for its two draws and jitter, the fighter's clothing foley set and volume; a fighter without a foley set never rustles (its clock is all ones).
- **Tangle break.** The hand parts are found by the human skeleton's hand bones (17, 18) and read in the drive's character frame in centimetres; the gate is the call's (no 0xA00 condition bit, muscles on). The clip starts once, from where the body is, and returns to the stance; an attack in progress is ended so its executor state does not linger.

## Risks

- A swing pose that keeps both hands behind the back for about 20 ticks starts Break Tangle mid-attack, as the original would; a frame origin that differs from the original's would change how often.
- The swing voice is quieter than the old whoosh (W25 × 0.07 × 0.1·√speed²).
