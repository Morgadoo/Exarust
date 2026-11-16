## ADDED Requirements

### Requirement: Torrent stirs cloths
A running torrent SHALL walk the level's cloths after the level contact: each live stream point whose reach (width × the reach scale the torrent's last contact left + margin + 40 cm) meets a cloth's bounding sphere and box SHALL move every cloth point within that reach along the point's last move by (1 − d²/r²)·0.01 and add (1 − d²/r²)·0.00001 to the cloth's stir; a cloth met SHALL be woken to 256 ticks.

#### Scenario: A cloth in the stream
- **WHEN** a duellist's torrent pours through a cloth
- **THEN** the cloth is untouched while it charges, and once it runs its points move, its stir rises and it is woken to 256

### Requirement: Force effect watch list
A blast's, wave's or barricade's sight entity SHALL join the simulation's watch list as it starts and leave it when the effect is no longer seen; leaving it SHALL make every brain forget the entity.

#### Scenario: A spent blast
- **WHEN** a brain keeps a blast among its contacts and the blast's chunks are spent
- **THEN** the blast leaves the watch list and the brain's contact with it goes, its other contacts staying

### Requirement: Torrent loops
A torrent's start SHALL take two process-stream draws for its charge loop's and stream loop's starts (length × 0.8 × draw, in double), and the voices SHALL take the tick's words: the charge loop at 0.4·(2 − c)·c while charging, then 0.01 less a tick until it is freed below 0.02, and the stream loop from pitch 6 easing toward volume 0.5 and pitch 0.6 while it runs.

#### Scenario: The loops' words
- **WHEN** a duellist's torrent charges and runs
- **THEN** the charge loop's volume follows the charge, then falls and stops, and the stream loop's pitch falls from 6 while its volume rises

### Requirement: Barricade sounds
A barricade's entry SHALL take a process-stream draw for its hum's start; the hum SHALL follow the frame's words at the node, and each impact the body test plays SHALL sound the `thmfrc_bi` set at the last struck sphere in front, carried into the world.

#### Scenario: A limb in front
- **WHEN** a character's limb stands 15 cm in front of a standing wall
- **THEN** one impact goes out at the limb's sphere, and none again within 400 ms

### Requirement: Wave alert
A force wave's throw SHALL alert the characters its caster's AI knows within 1800 cm as the blast's release does, its aim cleared first.

#### Scenario: A witness 3 m off
- **WHEN** a duellist's wave is thrown with a known character 3 m from its chest
- **THEN** that character's concern toward the caster rises by 0.25·(1 − (300/1800)⁴)·3 at the gain 3, and not while the wave charges

### Requirement: Force Strikes end sound
Force Strikes told to end SHALL play its end set at the caster's place at volume 0.3 and pitch 0.6 + 0.4·r, r the engine generator's next draw.

#### Scenario: Ending
- **WHEN** a running Force Strikes is told to end
- **THEN** one end set plays at that volume and pitch
