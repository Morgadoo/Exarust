## MODIFIED Requirements

### Requirement: Edge
A link SHALL store half its rest distance, and the edge pass SHALL move its first end by `f·d` and its second by `−f·d`, with `d` the second end less the first and `f = 0.5 − rsqrt(|d|²)·h` (the approximate reciprocal square root, `|d|²` summed `(x² + z²) + y²`). A link at its rest distance SHALL not move its ends; one stretched to twice its rest SHALL be pulled back to it.

#### Scenario: Rest and stretch
- **WHEN** a link of half rest 10 joins particles 20 apart, then one of half rest 5 joins particles 20 apart
- **THEN** the first pair stays put and the second pair ends 10 apart

### Requirement: Pin and wind
A pin of weight 1 SHALL place its particle on the rest point. While the world's wind is above 0.001 (compared in double), an awake cloth's particle SHALL move along the wind's direction by `(0.25 + n)·strength`, `n` the tile's noise where the particle was a tick ago; at or below it, SHALL not.

#### Scenario: Held and blown
- **WHEN** a pin of weight 1 has its rest 5 cm along X, then a particle is blown along X by a flat tile of 1 at strength 1, then at 0.001 in single, then at 0.0009
- **THEN** the pinned particle sits at 5 cm, the blown ones move 1.25 cm and 0.00125 cm, and the last stays

### Requirement: Rustle and mesh
A cloth whose record names a rustle sound SHALL rustle on an awake tick whose squared velocity sum exceeds 400, at volume `0.5·tail + 0.0001·|sum|` (the sum in double) and pitch `min(0.5·volume + 0.5, 2)`, and SHALL stop otherwise; the tail SHALL decay by 0.9 in double each awake tick. A record without a sound SHALL never rustle.

#### Scenario: Fast cloth
- **WHEN** a cloth with tail 0.2 moves 30 cm a tick, then is still, and a cloth without a rustle sound moves as fast
- **THEN** the first plays at `0.1 + 0.003` and then stops with its tail decayed twice, and the second never rustles

### Requirement: Mesh
A triangle mesh SHALL make one particle per position element, in element order (the vertices a seam splits SHALL share one particle). Its links SHALL be the walk out from the pins over the mesh's incidence ([Requirement: Cloth link walk]). A pinned vertex SHALL be held at its point. The face list SHALL contain both windings.

#### Scenario: Two triangles
- **WHEN** two triangles make a square pinned at one corner
- **THEN** its links are the four sides from the pin outward and then the two diagonals, the pin weight is 1, and both windings are listed

#### Scenario: A seam
- **WHEN** a square's two triangles are drawn with two of its four corners split across a UV seam, six vertices in all
- **THEN** the cloth has four particles and the walk's six links, and both copies of a split corner draw the same particle

### Requirement: Wind tile
Once a simulation step the wind hook SHALL set the strength to the scene's preset wind times 0.3 in double, draw once for each of the 256 samples, walk a sample within −1…1 by `(r − 0.5)·s`, above 1 by `−s·0.5·r` and below −1 by `+s·0.5·r` (`s = strength·0.2·4` in double; a NaN walks as in range), then normalise the direction and rebuild the tile's frame.

#### Scenario: One step
- **WHEN** the hook runs at scene wind 2 over samples 0, 1.5, −1.5 and NaN
- **THEN** the strength is 0.6, the samples take the three walks with the first three draws, the NaN stays NaN, and all 256 drew once

### Requirement: Cloth mesh write-back
Once a frame, after the frame's fixed ticks, a cloth's mesh SHALL take its particles' places between the tick before and the last, by the fraction of the next tick already elapsed (`a + (b − a)·t` in centimetres, then metres), and its normals rebuilt from the moved faces by the cloth pass's flat normal builder; the fixed ticks SHALL leave the mesh unchanged.

#### Scenario: Four ticks in one frame
- **WHEN** an awake hair cloth steps four ticks and the frame is drawn halfway into the next one
- **THEN** its mesh keeps its old positions through the ticks, and after the write-back holds each vertex halfway between its last two particle places, with the normals of those places

## ADDED Requirements

### Requirement: Cloth link walk
The links SHALL be walked out from the pins (vertex 0 without any): the open vertex taken is the last one whose distance is below the first open one's; it lists its fan's neighbours less each face's far end of its longest edge, links to each neighbour not settled, and pairs its neighbours unless the pair was linked from a settled vertex.

#### Scenario: One quad
- **WHEN** the walk runs over a square of two triangles from one corner
- **THEN** the links are, in order, that corner to its two sides, the next corner to the far one, the last side, and the two diagonals

### Requirement: Cloth link order
The edge pass SHALL visit the walk's links first, then the neighbour pairs whose arms' cosine is above 0.7 in double, then the other pairs; a vertex no pin reaches SHALL get no link.

#### Scenario: A strip of two quads
- **WHEN** a strip of two quads is walked from one end's two corners
- **THEN** the seven sides come first, then the pair in line through the middle vertex, and the diagonals last

### Requirement: Cloth tile sample
The tile's noise at a point of its frame SHALL be bilinear between the four samples about it, a cell being 200 cm (the fraction of `x·0.005` in double), wrapping past the last column and row to the first, summed in the original's order.

#### Scenario: Corner, inside and the wrap
- **WHEN** a tile holding each sample's index is sampled on a cell corner, inside a cell and past both edges
- **THEN** it gives the corner's sample, the bilinear blend, and the blend with the first column and row

### Requirement: Scene wind reaches the cloths
The simulation SHALL run the wind hook once a step before the cloth stage, from the live environment's preset wind (0 indoors), and every stepped cloth SHALL be blown by it in the world's frame.

#### Scenario: Outdoors and indoors
- **WHEN** a loose cloth steps 20 ticks under a preset wind of 2 and another under none
- **THEN** both hooks drew 20 × 256 times, the first cloth drifted along +x and +z, and the second did not

### Requirement: Cloth integrate order
The integrate SHALL add `((v + (v̄ − v)·drag)·keep + g) + x` to each particle, in that order.

#### Scenario: One moving particle
- **WHEN** a particle with a velocity is stepped under gravity
- **THEN** its new place is bit for bit that sum

### Requirement: Cloth rustle sound
The cloth stage SHALL send a cloth's rustle while it plays and once when it stops; the sound SHALL retrigger the cloth's voice on a variant of its set at pitch 0.5 when the voice has ended, then place, level and pitch it, and SHALL set its stop bit when the rustle stops.

#### Scenario: Thrown and stilled
- **WHEN** a level cloth moving 40 cm a tick in all steps, then is stilled for two steps
- **THEN** one playing rustle at volume 0.004 is sent, then one stop, then nothing
