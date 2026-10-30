# effect-nodes Specification

## Purpose
The original's small effect nodes: the lit blade's flame row, the teleporter
scan, the teleport bubble, the displacement shell, the power screen's domain
wheel, the level portal's light, dissolve burst and spray, and the force and
environment effect nodes: the rain, the force domes, the Force Wave, the
vanish, what the AI sees of the force effects and their GI lights.

## Requirements

### Requirement: Effect clock
The system SHALL step the small effects on the original's game clock in whole milliseconds and SHALL take a span between the clock and a stamp as a signed difference of the two widened words.

#### Scenario: Two ticks
- **WHEN** two 83 Hz ticks of game time have passed
- **THEN** the clock reads 24 ms

#### Scenario: A stamp ahead
- **WHEN** a shell is stamped 100 ms after the clock
- **THEN** its shown fraction is −0.25

### Requirement: Domain wheel
The system SHALL place the domain wheel's six icons clockwise from the top of a 76 px ring, Mind, Force, Body, Energy, Light and Displacement, and SHALL hit an icon when the squared whole-pixel distance to its rounded place is under 433.

#### Scenario: The first icon
- **WHEN** the wheel's centre is (100, 100)
- **THEN** icon 1 is at (100, 24) and icon 2 at (166, 62)

#### Scenario: The hit edge
- **WHEN** the mouse is 12 px right of and 17 px below icon 1
- **THEN** no icon is hit

### Requirement: Displacement shell fade
The system SHALL give the displacement shell's shader the fade (1 − shown²)².

#### Scenario: A quarter shown
- **WHEN** the shell is a quarter shown
- **THEN** the fade is 0.87890625

### Requirement: Teleport bubble fade
The system SHALL upload the bubble's own value as its fade while it shrinks, and max(1 − value, 0) before.

#### Scenario: Shrinking halfway
- **WHEN** a shrinking bubble's value is −0.5
- **THEN** its fade is −0.5

### Requirement: Flame row lag
The system SHALL move each lagging point of a flame row a fifth of the way to a point 20 cm above its end on the up axis each tick.

#### Scenario: From the origin
- **WHEN** a lagging point at the origin follows an end at (10, 0, 0)
- **THEN** it moves to (2, 4, 0)

### Requirement: Portal dissolve seeding
The system SHALL jitter each seeded dissolve particle's position by up to 1.5 cm per axis and push it 1.5 % further from the body's centre, keeping its previous position at the cell's point.

#### Scenario: The first cell
- **WHEN** seed 1 seeds a 12 cm cube at the origin around a body centred at the origin
- **THEN** the first particle's previous position is (−6, −6, −6) and its position x is about −6.343

### Requirement: Level portal light
The system SHALL give a loaded level portal marker a light of colour (0.49, 0.69, 1), intensity 0.75, radius 500 cm and softness 3, 50 cm above it.

#### Scenario: A loaded marker
- **WHEN** the game sees a new portal marker node
- **THEN** one portal light hangs under it 0.5 m up

### Requirement: Rain drops
The system SHALL keep 4096 rain drops in a box 300 cm to each side and 600 cm tall, centred 450 cm ahead of the eye along the camera's negated backward axis, and SHALL draw the first round(4096·intensity) vertices of the heads and tails.

#### Scenario: A quarter intensity
- **WHEN** the intensity is 0.25
- **THEN** 1024 vertices are drawn and the drops past the 1024th do not move

### Requirement: Rain update
A rain update SHALL move each drawn head by (0, −1.5·f, 0) with f = (0.4·intensity + 0.25)·ms, put a head whose height has its sign bit set back up by 600 cm with x and z from the effect's own generator words, and set its tail at the head plus the move times −(8·intensity + 8)/(−f′/(y⁴ + 0.2)), f′ being f capped at 30 and y the backward axis's height.

#### Scenario: A level 12 ms update
- **WHEN** a rain at intensity 1 is updated after 12 ms with a level view
- **THEN** each drawn head falls 11.7 cm and its tail lies 4.8 cm below it

### Requirement: Rain colour
The rain colour SHALL be the environment's colour times 2.2, times 0.075·intensity + 0.075 (0.03·intensity + 0.03 while the game is paused), over 255, and its alpha (0.5·intensity + 0.5)·(|y| + 0.5).

#### Scenario: Full rain looking down
- **WHEN** the intensity is 1, the environment white and the backward axis's height −0.5
- **THEN** the red is 2.2·0.15/255 and the alpha 1

### Requirement: Portal spray particle
A portal spray SHALL advance its counter before it fills a slot, wrap past the count (clearing its ring flag) unless closing, and place each particle on an ellipse of 80·s·scale by 115·s·scale cm (s = 0.8 + 0.4·r) at the node's place, its previous position the point and its position 1.0015 times the point.

#### Scenario: The first particles
- **WHEN** a spray of three particles spawns four times
- **THEN** it fills slots 1, 2, 3 and then 0, clearing its ring flag on the wrap

### Requirement: Dome ripples
A strike on a dome SHALL be capped at 5 and SHALL go to the ripple whose target lies strictly nearest within the reach, adding its point to that ripple's sum and keeping the larger strength, or else open a new ripple at the point while fewer than 8 exist. Each tick SHALL set the reach to 30 + 1.25 per ripple, move a struck ripple's target to the mean of its strikes (half way once shown), step every ripple and replace one with both values under 0.07 by the last.

#### Scenario: Two strikes in reach
- **WHEN** a dome is struck at (100, 0, 0) with 9 and at (110, 0, 0) with 1, then ticks
- **THEN** one ripple holds strength 5, its target is (105, 0, 0) and the reach is 31.25

### Requirement: Barrier phases
The Barrier SHALL grow at 0.00125 of its share a millisecond only while under 800 ms have passed since its start, drawing a radius of (2 − s)·s·0.9·r, SHALL flip its share and keep the matching clock when told to end, and SHALL shrink at 0.002 a millisecond with a drawn radius of (1 − s²)·0.9·r, passing to its end above 1. Its height SHALL follow the higher of the caster's height plus 10 and the ground plus 100 as h + (h − h′)·0.95 + (target − h)·0.003.

#### Scenario: The grow window
- **WHEN** a Barrier started at 1000 ms ticks at 1400, 1799 and 1900 ms
- **THEN** its share is 0.5, then just under 1, and stays there with no power-used report

### Requirement: Barrier cap
The Barrier's drawn cap SHALL be scaled 85 by 85 by 25 cm, stand 75 cm along its facing with a bounding radius of √(25² + 85²), and push a body part that is within its radius plus the part's, not below half the radius, inside the drawn radius sideways and outside the inner surface outward, opening a ripple of half the push's length.

#### Scenario: A part ahead
- **WHEN** a part of radius 5 stands 95 cm straight ahead of a 100 cm Barrier whose drawn radius is 90
- **THEN** a ripple opens at the shell point straight ahead

### Requirement: Bubble keep-out
The Bubble SHALL push its caster's parts that reach 10 cm inside its surface back in by a tenth of the overlap over the distance, and other bodies' parts its shell covers out by (0.1 − 0.01·min(overlap, 1))·d scaled by the mass factor (at most 0.01).

#### Scenario: The caster at the edge
- **WHEN** the caster's part of radius 5 stands 88 cm from the centre of a 100 cm Bubble
- **THEN** it is pushed inward and ripples with strength 3

### Requirement: Force Wave
The Force Wave SHALL stand 32 chunks 80 cm out and 30 cm up round the caster, each with a twelve-slot trail headed at slot 10, SHALL release the blast at 1200 cm when its charge passes 1, and once thrown SHALL move each chunk by its last move plus 2.4·(1 − charge)²·weight along its direction, times 0.96, ending when every chunk's 58-tick life has run out.

#### Scenario: The first thrown tick
- **WHEN** a wave at charge 0.5 takes its first thrown tick
- **THEN** each chunk moves 0.576 cm along its direction and its trail head moves to slot 11

### Requirement: Vanish phases
The vanish SHALL hide bodies by clearing render bit 4 and their shown amounts, show them by setting the shown amounts to 1, show again (phase 4) at each phase end until the phase table runs out, and SHALL keep running while its own phased body's fade counter is under 4.

#### Scenario: A lent body
- **WHEN** a vanish lent a phased body with flags 0x31 finishes
- **THEN** the body's flags are 0x1 and the bodies are hidden

### Requirement: Force danger sight
An effect within 1200 cm of a viewer's chest SHALL set flag 4, the urge min(1 − d/1200, 0.99) and the near score 36·s (blast), 12·s (wave) or 6·u (barricade, measured from its nearest wall point), s = (2 − u)·u, doubled when released; the face word SHALL be ((look·dir)·−½ + ½)²·¼ + ((facing·dir)·−½ + ½)·¾.

#### Scenario: A blast six metres ahead
- **WHEN** a viewer facing a blast 600 cm ahead assesses it with relation 0
- **THEN** the urge is 0.5, the near score 27 and the face 0

### Requirement: Force emissive values
The force effects' GI lights SHALL take the Barrier's colour (0.14, 0.08, 1)·min(1.5·share, 1)·gain·0.8 while growing, the Bubble's radius plus 5, the cuboid's extent max(scale − 20, 10) per axis, and the teleporter bubble's colour (0.75, 0.8, 1) times the gain and its swell.

#### Scenario: A cuboid's extent
- **WHEN** a cuboid node is scaled (25, 50, 31)
- **THEN** its light's extent is (10, 30, 11)

### Requirement: Blast and torrent geometry
The blast's and the torrent's geometry SHALL use `effectforce_v` with `effectforce_f` and `effecttorrent_f`, bind `ObjMat`, `TexOfs` and `EmiTex`, and draw their passes on the 0.002 row step; their texture offsets SHALL be `(0, r, texture scale)` with the first chunk's radius between its last two ticks, and `(0, scroll, 1)`.

#### Scenario: The torrent's program
- **WHEN** the torrent's geometry class is read
- **THEN** its fragment program is `effecttorrent_f` and its vertex program the tendril's

### Requirement: Blast light
The blast's light SHALL put one point a chunk coloured (0.16, 0.08, 1) times the gain, its strength and the setup's light; the strength SHALL be `(0.6 − c)·c·11.11` under a charge of 0.3 and, thrown, `max(4 − 4.5·f, 0)²` above a spent share of 0.666, and the attenuation 5000 while charging and, thrown, `5000 − 13333·f` under 0.3 and 1000 from it.

#### Scenario: A late chunk
- **WHEN** a thrown chunk has a third of its life left
- **THEN** the attenuation is 1000 and the strength `(4 − 4.5·⅔)²`

### Requirement: Torrent light
The torrent's light SHALL put one point a gathered sample coloured (0.12, 0.06, 1) times the gain, its weight over 4 and `(2 − l)·l·1.2`, attenuation 2000; its samples SHALL merge into the nearest point within 80 cm, or within 160 cm once sixteen are kept.

#### Scenario: A torrent at full level
- **WHEN** a torrent at level 1 lights a point of weight 2
- **THEN** its colour is (0.12, 0.06, 1) times 0.6

### Requirement: Effect material pass draws
Each of the four effect material passes SHALL draw nothing while the global pass count is zero or less. Otherwise it SHALL set layer 0's colour and draw the first entry point, then for each layer from 1 to the count set that layer's colour and draw the layer entry point.

#### Scenario: No pass count
- **WHEN** any of the four passes runs with a pass count of 0
- **THEN** it draws nothing

### Requirement: Effect material depth terms
Each layer's matrix depth term SHALL be `((layer + frame + offset) × step) / divisor + base − step/2`, the counters added as integers. The effect material and tendril SHALL use step 0.002 and half step 0.001 in double precision; the shader and shell the sub-object's level of detail in single precision, with offset 1.5 for the shell and 0.5 otherwise.

#### Scenario: Depth term in double
- **WHEN** the effect material pass plans layer 0 at frame 10 from a base of 0.7
- **THEN** the depth term is 0.71999997, not the single-precision 0.72

### Requirement: Effect material pass uniforms
The shader and shell passes SHALL bind a time `base + layer × step` and, while display bit 0x40 is set, a brightness `(light − layer/divisor) × scale`. Only the shader pass SHALL bind the texture coordinate `(layer + frame + 0.5)/divisor − 0.5`, and only when its location exists. The effect material and tendril passes SHALL bind neither.

#### Scenario: Divisor, not pass count
- **WHEN** the shader pass draws layer 1 of 2 at frame 0 with light 1 and scale 2
- **THEN** the brightness is 0 and the texture coordinate is 1

### Requirement: Effect material node words
The force and tendril materials SHALL set bit 0x20 in the node's flags word. The aura and shell materials SHALL set bits 0x9000 in the node's state word and replace its sub-objects with one sub-object of buffer usage 35044 whose parent is the node.

#### Scenario: Aura node
- **WHEN** a Shell cast spawns its aura node
- **THEN** its state word carries 0x9000, it has one sub-object and binds `_ForceNoise`

### Requirement: Displacement shell pass bits
The shell pass's segment bit 0x200 SHALL draw every node's parts lacking 0x80040 with colour writes off and depth writes on, thick when the node's flags word carries 0x8000. Bit 0x400 SHALL set every skin's and node's fades to one less the shown value and draw each part lacking 0x80000 with its material's fade at the shown value and a 0x100 stage switched to 0x203, then restore both.

#### Scenario: Fade restored
- **WHEN** the fade pass draws a part whose fade is 0 at a shown value of 0.25
- **THEN** the draw uses 0.25 and the part's fade is 0 afterwards

### Requirement: Long Door placement
A Long Door's move SHALL place the body's root at the destination (centimetres) converted to metres, raised by the root's height above the body's lowest part.

#### Scenario: Destination in centimetres
- **WHEN** a body with its root 0.9 m above its lowest part moves to (500, 20, −300) cm
- **THEN** its root lands at (5, 1.1, −3) m

### Requirement: Live rain
While the scene's rain effect exists and the rain amount is above 0.01, the game SHALL step the rain on the game clock with the eye and the camera's backward axis, give it the scene's scratch colour, write its drift figure back as the rain sound's volume and its fall figure as the sound's pitch, and draw its streaks; the effect SHALL be freed with the scene's rain effect.

#### Scenario: Still gust at 0.9
- **WHEN** the rain at intensity 0.9 is updated with a gust of 0
- **THEN** the rain sound's volume becomes 0.1215 and its pitch 1.21

### Requirement: Rain streak vertices
The streaks SHALL be drawn as lines one window pixel wide (twice, one target pixel apart, on the 2× supersampled target) from the box turned 2° about its x axis and scaled by √(fov/70), with the drops' colour bytes read unnormalised (grey 160, alpha 0 at the head and 255 at the tail) times the rain colour, the alpha faded by (0.6 − 0.000667·d)⁴ with d the eye distance in cm.

#### Scenario: At the eye
- **WHEN** a tail vertex lies at the eye with a rain alpha of 1
- **THEN** its alpha is 255·0.1296

### Requirement: Rain streak fragment
Each streak fragment SHALL be the frame behind it t, clamped to 1, as 0.75·t + 3·max(t − 0.1, 0)² plus the vertex colour, clamped to 1 and blended with source alpha over one minus source alpha. A view without a copy of its frame SHALL draw no rain.

#### Scenario: Bright sky behind a streak
- **WHEN** a streak with colour 0.1 and alpha 0.5 lies over a frame value of 0.5
- **THEN** the fragment is 0.955 and the blend leaves 0.7275
