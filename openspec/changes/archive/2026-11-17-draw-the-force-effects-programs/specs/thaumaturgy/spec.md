## ADDED Requirements

### Requirement: Force strip in every state
A blast's or wave's strip SHALL be built every frame from its live chunks whatever its state, a charging effect's chunks in the charge's colour; a strip whose points are not finite SHALL not be drawn that frame.

#### Scenario: A charging blast
- **WHEN** a duellist's blast has charged three ticks
- **THEN** its strip is drawn with the force program and its chunks carry the charge byte

### Requirement: Wave draw and light
A force wave SHALL be drawn and lit as a blast of its 32 chunks: the blast's strip with its texture offset, and the blast's multi-point GI light over the life records' light points the frame writes, freed on the frame that builds nothing.

#### Scenario: A charging wave
- **WHEN** a wave has charged three ticks
- **THEN** its strip is drawn and its light carries 32 finite points

### Requirement: Arc visible gate
An electricity emitter whose node is outside the scene camera's view SHALL keep its voxel light and hum but build no scroll, ribbon or light points and SHALL not put its tendrils' origins back that frame.

#### Scenario: Off screen
- **WHEN** an emitter with lit points draws unseen
- **THEN** its ribbon is empty, its points' light is 0 and a voxel light still shines

### Requirement: Barricade program
The barricade SHALL draw its slid slab, unscaled, through the `effectfbrcd` program with `ViewPos` the camera in the node's frame, `BExt` the node's half extents, `TexPos` the slid texture place, the eight ripple points and `Fade` from the rise or fall, over the `_ForceNoise` picture with the barrier record's premultiplied blend.

#### Scenario: Rising
- **WHEN** a barricade rises at share t
- **THEN** its `Fade` is ((1 − t)²)² and `BExt` is the drawn half width and height

### Requirement: Displacement tendril smoke program
The displacement tendrils SHALL draw through their geometry's own program, `effectforce_v` with `effectsmoke_f`, over the `_TDispMat01` record's `FlameletsA3` layer, with `TexOfs = (0, scroll, 1)` (the effect's scroll between its last two ticks), the strip's colour bytes and its two coordinate sets, blended ONE / ONE_MINUS_SRC_ALPHA on both faces with no shadows.

#### Scenario: A tendril set
- **WHEN** a lit tendril set has ticked four times
- **THEN** its material is the smoke program with the shown scroll in `TexOfs` and its strip carries the second coordinate set

## REMOVED Requirements

### Requirement: Displacement tendril material
**Reason**: The `_TDispMat01` record's draw method draws the strip through the tendril geometry's own `effectsmoke_f` program, not the surface program.
**Migration**: See Displacement tendril smoke program.
