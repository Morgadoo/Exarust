## ADDED Requirements

### Requirement: Offscreen lockstep readback
A `--fixed-frames` offscreen game SHALL wait for the GPU at the end of each frame, so each frame reaches the frame file exactly one frame after it was drawn, with its loading flag and counter the same in every run. Without `--fixed-frames` the readback SHALL stay asynchronous.

#### Scenario: Two runs number their frames alike
- **WHEN** `play arena --offscreen f.bin --fixed-frames --frames 300` runs twice
- **THEN** the frame the loading flag clears on has the same counter in both frame files

### Requirement: Offscreen exact shots
An offscreen game given `EXA_OFFSCREEN_SHOTS=F1,F2,…` SHALL save the read-back frames that many frames past the loading screen (the first frame without the loading flag is 1) as `<frame file without its extension>-fNNNNN.png`.

#### Scenario: A runner's shot
- **WHEN** `EXA_OFFSCREEN_SHOTS=150 exarust play arena --offscreen /r/a.bin --frames 300` runs
- **THEN** `/r/a-f00150.png` holds frame 150 past the loading screen

### Requirement: Fixed-frame pipelines
A `--fixed-frames` run with a renderer SHALL compile each new pipeline in the frame that asks for it, so a mesh needing a new pipeline variant is drawn from that frame in every run.

#### Scenario: A blasted body
- **WHEN** the golden blast scene runs three times with `--fixed-frames`
- **THEN** its shots after the blast are the same in all three runs

### Requirement: Fixed-frame background jobs
In a `--fixed-frames` run a background job a frame polls (texture decodes, collider, limb-tree, GI, navigation, terrain, dressing and icon builds) SHALL be waited for, the content stream SHALL finish what was posted before its pump, and the load's per-frame time budgets SHALL be lifted. A run without the option SHALL poll as before.

#### Scenario: The loading screen's length
- **WHEN** a golden scene runs twice with `--fixed-frames`
- **THEN** a job's result lands in the same frame in both runs

### Requirement: Frame digest
`--frame-digest <file.csv>` SHALL write one row a frame past the loading screen with hashes of the cameras, lights, drawn meshes, surface materials, images and GI state, entity and asset ids left out, and `EXA_FRAME_DIGEST_DUMP=F1,F2` SHALL list those frames' items in full beside it.

#### Scenario: Two runs compared
- **WHEN** two `--fixed-frames` runs of one scene write digests
- **THEN** `context/tools/frame_digest_diff.py` names the first frame each column differs, or none
