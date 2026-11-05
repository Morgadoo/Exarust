# Tasks

- [x] Trace the initialiser, the context opening, the EFX enable, slot and effect creation, the upload, the decay setter, the source bind and every caller of the enable in REA; check the enable callers' argument and the send call in the assembly.
- [x] Check the toolbox for OpenAL Soft (absent; no sudo) and choose the port.
- [x] `exa_audio::efx`: the EAX reverb and the source send; tests for the send mapping, the taps, the shelves and the decay.
- [x] `exa_audio::efx_bus`: the bus source, the mirrored commands, the level log and the recording; tests.
- [x] `exa_audio::reverb`: the effect's block with the decay range, `LevelReverb::for_level`, the per-level log; tests.
- [x] `exarust::viewer`: outdoor arenas turn the reverb off.
- [x] Crate README.
- [x] GPU checks: `play exanima01` (reverb on, levels and recording), `play arenao3` (reverb off, silent bus), `arena1`, `arenahub`, `exanima02` (offsets logged).
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.
