# Proposal

## Why

Wiring backlog S7. The original plays every world sound through OpenAL Soft
1.21.1 (`OpenAL64.dll`) with an EFX EAX reverb send, so its rooms ring.
Exarust computed the reverb block and each voice's send flag but played
through Bevy's rodio backend, which has no effect slots or sends: no reverb
was heard, and the outdoor arenas were not told apart from the rooms.

Phase: Phase 5 audio of docs/PLAN.md (no PLAN.md edit in this change).

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly): the
lazy audio initialiser (listener gain 1, 64 sources with rolloff 1,
reference 400 and maximum 3000 units, the linear clamped model, no metres
per unit, speed of sound or air absorption set), the device and context
opening (no context attributes), the EFX enable and its slot and effect
creation, the property upload (one `alEffectf`/`alEffectfv`/`alEffecti` per
field), the decay offset setter (double-precision `offset + 1.49`, upload
and reattach while on), the source bind (auxiliary send 0 to the slot, no
filter, only at bind time and only for world voices while the reverb is on),
and every caller of the enable: the scene builders and the indoor arena
preset turn it on, the outdoor arena preset (game and scene editor) turns it
off. The shipped `oalsoft.ini` sets only `hrtf = false` and
`front-stabilizer = true`. The reverb itself follows OpenAL Soft 1.21's
EAX reverb and source send code.

## What Changes

- `exa_audio::efx`: the port of OpenAL Soft's EAX reverb (input band
  shelves, main delay and early taps, early all-pass and delay, four-line
  late feedback network with T60 shelves and all-pass, first-order panning
  to stereo) and of the source's send (`wet_send`: the clamp, room rolloff
  and the reverb's initial decay past the reference distance in the
  engine's centimetre units, with its HF and LF band gains).
- `exa_audio::efx_bus`: the reverb slot as a Bevy audio source. The
  playback commands of every sending source slot are mirrored onto a wet
  copy (same buffer, pitch, start, pause, restart and stop), filtered by its
  send gains and summed into the reverb on rodio's audio thread. Flags:
  `EXA_EFX_LOG` (levels once a second), `EXA_EFX_RECORD` /
  `EXA_EFX_RECORD_SECS` (the reverb's output to a WAV).
- `exa_audio::reverb`: the block the effect holds (`Reverb::effect`), with
  OpenAL's refusal of a decay outside 0.1 to 20 s; `LevelReverb` carries
  whether the level leaves the reverb on; the parameters are logged per
  level.
- `exarust::viewer`: an outdoor arena (`arenao*`) turns the reverb off.

## Out of scope

- OpenAL Soft itself: the fedora-toolbox-44 toolbox has no `openal-soft`
  library and packages cannot be installed without sudo, so the reverb is a
  port in the existing mixer, not a binding.
- The source's direction into the reverb's four inputs (the send is
  omnidirectional here), the dry path's ambisonic panning (rodio pans the
  dry sources), OpenAL Soft's crossfade when the block changes, and an A/B
  against the retail game's recorded output.
