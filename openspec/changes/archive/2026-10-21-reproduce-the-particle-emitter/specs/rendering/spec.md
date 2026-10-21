## MODIFIED Requirements

### Requirement: Fires
The system SHALL turn model nodes with a fire emitter into fires per the original's presets. A fire of size s up to 0.35 has one emitter at the node and a light of sRGB (1, 0.68, 0.43), intensity 8√s and radius 1500√s cm (×1.25 above 0.2, compared in double precision), 5 cm above the node. Its intensity follows the emitter's smoothed live count and a soft-bounded random walk.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown
- **THEN** its two torches light the walls around them in warm light

#### Scenario: A torch's light
- **WHEN** a torch's stored size 0.2 is turned into a fire
- **THEN** its light has intensity 3.58 and radius 838.5 cm

### Requirement: Flame particles
The system SHALL draw fires' flames as the original's particles. A small fire's particles start at rest on a ring around the node, rise by buoyancy at the 83 Hz tick, are pulled toward the node's axis and die young or past the fire's reach. Each is a diamond stretched along its motion, facing the line from the camera to the particles' centre, coloured by the fire's keys, blended premultiplied. A hidden flame builds no quads.

#### Scenario: Menu torches
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** each torch shows a rising, tapering flame about as tall as the original's

#### Scenario: A particle at rest
- **WHEN** a torch emits its first particle at a node 50 m from the origin
- **THEN** the particle's position and previous position are equal

### Requirement: Released electrified effect
The system SHALL release an electrified item's arc by making its first arc child (kind byte 0x21) stop renewing and enter state 1, and SHALL clear the owner's hit and release callbacks whether or not a child matched.

#### Scenario: Released and other
- **WHEN** an item with a 0x22-kind child and a 0x21-kind child is released
- **THEN** the 0x21 child stops renewing and is in state 1, the other is unchanged, and the owner's callbacks are gone
- **WHEN** an item with no arc child is released
- **THEN** only the owner's callbacks clear

## ADDED Requirements

### Requirement: Fire tiers
The system SHALL give a fire above size 0.35 a flame on a ring of round(√s·n) sources and a bed below it: 0.35–0.5 with a light 40 cm up (8/1500), 0.5–1 with one 50 cm up (12/1600), and above 1 the large pair at half the size with one 40 cm up (12/1500), whose emitters are one-shot and whose fire is removed once they finish.

#### Scenario: A medium fire
- **WHEN** a fire of size 0.8 is built
- **THEN** its flame has 4 sources on a 32 cm ring and its bed sits 16 cm below

#### Scenario: A large fire burns out
- **WHEN** a fire of size 2 has emitted its ring and its last particle has died
- **THEN** the fire and its light are removed

### Requirement: Particle emitter tick
The system SHALL tick a particle emitter at 83 Hz as the original does: the light walk and density, the ageing, the head pops, the burst refund, Verlet integration, the one-shot or finite check, the accumulator losing one two ticks in three, then spawning while it is below one, adding (83/rate)·⅔ after each round of sources.

#### Scenario: A torch's derived values
- **WHEN** the small template is scaled to 0.2
- **THEN** its rate is 50, its lifetime 103 ticks, its pool 97 and its density normaliser 25

#### Scenario: The distance cap
- **WHEN** a torch particle with 103 ticks left stands 35 cm above its node
- **THEN** its life is capped to 13

#### Scenario: An emptied ring
- **WHEN** the last two particles of a ring die in one tick
- **THEN** the tick counts three pops

### Requirement: Particle spawn
The system SHALL spawn a particle with the Verlet pair moved by the template's per-tick speed along its random direction, and its previous position lagging by 0.3 of the source's motion. Integer draws SHALL come from the engine's linear congruential sequence and unit draws from the run-time library's Mersenne twister, in the original's order.

#### Scenario: A moving source
- **WHEN** a torch's source moves 10 cm in the tick a particle spawns
- **THEN** the particle's previous position lags it by 3 cm

### Requirement: Fire colour keys
The system SHALL colour fire particles from the original's key tables as the linear pipeline loads them: the RGB of keys 1…n scaled by 0.7 through a float and back, rounded half to even, the alpha untouched, the 0.35–0.5 bed's table unscaled. A particle's colour SHALL be the keys at n·age lerped per channel and rounded half to even, odd ring slots one key later.

#### Scenario: The small fire's brightest key
- **WHEN** the small fire's key (255, 160, 94, 64) is loaded
- **THEN** it becomes (178, 112, 66, 64)

#### Scenario: The bed's peak
- **WHEN** a 0.5–1 bed particle is half way through its life on an even slot
- **THEN** its colour is (126, 70, 35, 0)

### Requirement: Carried fire
The system SHALL carry a held torch's fire with the hand as an emitter attached to the item's body: each tick its live particles move by half the hand's displacement and new particles inherit 0.3 of it.

#### Scenario: Walking with a torch
- **WHEN** the player walks carrying a lit torch
- **THEN** the flame follows the hand with a short trail instead of being left behind

### Requirement: Electric arc presets
The system SHALL build arcs from the original's presets: an electrified weapon of 20 sustained tendrils of 5 segments of 6 cm, 4 cm wide, renewed every 10–19 ticks, with a voxel light (0.6, 0.6, 1) of 16 at 180 cm; a spark burst of 26 one-shot tendrils of 5 segments of 24 cm, 6 cm wide; a level device of size d/1000 with round(l/(4·size)) segments and a point light of size/2 at 80·size cm.

#### Scenario: The weapon's ribbon
- **WHEN** a weapon arc's ribbon is built
- **THEN** each segment quad is 4 cm across, with V running across it

#### Scenario: A device
- **WHEN** a device of duration 1000 and length 200 is built
- **THEN** it has 50 segments, 10 tendrils and a point light of 0.5 at 80 cm

### Requirement: Electrified charge
The system SHALL raise an electrified item's charge by 10 a hit, or 0x80 for a part kind with bit 0x10, and the step that would reach 255 SHALL fill it and release the arc.

#### Scenario: A charge's last step
- **WHEN** a charge of 245 takes a normal hit
- **THEN** it becomes 255 and the arc is released

### Requirement: Arc record
The system SHALL read the 84-byte arc record's flags, counts, tendril lifetime in ticks, size, segment scale, tendril values, sound name (none when empty), jitter (0.1 when not positive), offset, direction and drift. A short record SHALL be an error, never a panic.

#### Scenario: A short record
- **WHEN** a 10-byte record is read
- **THEN** it is an error

### Requirement: Arc record flags
The system SHALL make a record's arc sustained, renewing every n + Random(n) ticks, when flag 1 is set, and one-shot over n ticks otherwise. Flag 0x200 SHALL add the weapon's voxel light; 0x300 SHALL register an electric flow instead of a light.

#### Scenario: A sustained record
- **WHEN** a record with flags 0x201, lifetime 25, size 1 and scale 0.5 is read
- **THEN** the arc is sustained, renewing every 25–49 ticks, with 2 cm segments, 2 cm wide and the weapon's voxel light
