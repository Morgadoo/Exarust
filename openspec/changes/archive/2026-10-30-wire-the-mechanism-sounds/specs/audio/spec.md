# Spec Delta

## ADDED Requirements

### Requirement: Mechanism creak voices
A door's hinge creak (`door` + its hinge letter + `_hng`), a lever's creak (`leverb_mov`) and a push surface's grind (`Stnb_mov`) SHALL each keep one voice on the body. On a tick that creaks, a voice that is not playing SHALL restart on a fresh variant of the set at the body's centre with the tick's volume and pitch, without the loop flag, and a playing voice SHALL only take the new volume and pitch. A quiet tick SHALL set the voice's volume to 0, ask it to stop and free it.

#### Scenario: A pulled hinge
- **WHEN** a hinged door's tick reads a squared handle speed of 4
- **THEN** its creak voice plays at volume 0.075 × √2 and pitch 0.744

#### Scenario: Still playing
- **WHEN** the creak's voice is playing and the next tick creaks again from elsewhere
- **THEN** the same voice takes the new levels and keeps its first point

#### Scenario: Quiet
- **WHEN** a creaking door's next tick is quiet
- **THEN** its voice is stopped and freed

### Requirement: Door one-shots
A door's knock, slam (`slm`) and unlatch (`opn`) SHALL play a variant of the door's set at the handle point the step measured, a signalled open's unlatch at the point measured the tick before. A hinged door's knock SHALL play the body's own collision set when it has a variant, else the slam set; a sliding door's knock SHALL play the slam set.

#### Scenario: A hinged knock
- **WHEN** a hinged door whose body has no collision set knocks
- **THEN** a variant of its slam set plays at its handle point

### Requirement: Mechanism one-shot draws
Each mechanism one-shot's random share SHALL be drawn pitch first, then volume, each only when it has a share, and formed in double precision. A push surface's click SHALL play `Stnb_fit` at its centre at volume 0.3 and pitch 0.9 + 0.2 × draw.

#### Scenario: The unlatch
- **WHEN** a pulled door unlatches with draws 0.5 then 0.25
- **THEN** `opn` plays at pitch 0.95 and volume 0.1125

#### Scenario: A knock
- **WHEN** a door knocks
- **THEN** one draw is taken, for its pitch

#### Scenario: The click
- **WHEN** a plate goes down and the draw is 0.5
- **THEN** `Stnb_fit` plays at volume 0.3 and pitch 1.0

### Requirement: Lever return tick loop
Arming a lever's timed return, at its report or at load from a saved return, SHALL start `leverb_tck01` on a fresh voice at the lever's centre, looping, at volume 0.1 and pitch 1. While the lever returns its volume SHALL follow the step's fade, and the voice SHALL be freed when the fade ends or the lever reports 0.

#### Scenario: Armed then faded
- **WHEN** a lever arms its return and later fades to 0.09
- **THEN** its loop starts at 0.1 and then plays at 0.09

### Requirement: Door lock sounds
A door's lock sounds SHALL play its numbered sets: `LkO` for the open order's first door and the unlock with the right key, `LkT` for another item on a latched door and for the hand's press on a latched door, at the handle point. The unlock and the rattle SHALL draw pitch 0.8 + 0.2 × draw, then volume 0.15 + 0.1 × draw.

#### Scenario: The rattle
- **WHEN** the hand presses a latched door and both draws are 0.5
- **THEN** its `LkT` set plays at volume 0.2 and pitch 0.9
