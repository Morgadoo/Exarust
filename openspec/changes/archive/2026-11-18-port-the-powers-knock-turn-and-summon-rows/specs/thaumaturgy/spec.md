## ADDED Requirements

### Requirement: Force contact knockback step
A character struck by a blast's or wave's chunks SHALL first lose its knockback kind; a struck character whose gathered impulse is longer than 1 SHALL take the knockback step routine: the impulse's direction with its height zeroed, times four times its length capped at 30, cut to its step length, the step starting at its chest plus that push. A torrent's struck character SHALL take the same routine without the clear.

#### Scenario: A hard blast
- **WHEN** a blast's chunks strike a character's part with an impulse of 25
- **THEN** the character's knockback step targets its chest plus at most one step length along the impulse, flat

### Requirement: Torrent turned aim
A torrent SHALL keep a turn spring reset at its start's aim heading; each tick the spring SHALL follow the aim's heading and take one step, and the torrent's frame SHALL look along the spring's flat direction.

#### Scenario: The caster turns a quarter
- **WHEN** a torrent's aim turns a quarter turn
- **THEN** its frame swings round over several ticks and settles on the new heading

### Requirement: Revival voice
Raise Dead's revival of a corpse SHALL pick the character's voice names again from its record after the feral level is set, keeping the voice's volume, gain and pitch.

#### Scenario: A living voice comes back undead
- **WHEN** a corpse with a living voice is revived
- **THEN** its voice's groans, grunt and scream are the undead set's

### Requirement: Ending dome hum
An ending dome's frame SHALL write no base hum, keeping the hum voice's words from the last frame, and SHALL add the glow to them again each frame.

#### Scenario: Two ending frames
- **WHEN** a dome draws two frames while ending
- **THEN** each frame's hum is the last one's plus the glow once on the volume and 2.25 times on the pitch

### Requirement: Summoned demon bring-back
A summon whose held phase ends SHALL bring its demon's body back as a vanished body comes back: enabled, not kinematic and settling for 1000 ticks; its release SHALL end the settling before the wake.

#### Scenario: The fade starts and ends
- **WHEN** a summon's shell starts to open and later its fade ends
- **THEN** the demon's body settles for 1000 ticks, and at the release its settling is 0 and its AI is no longer held
