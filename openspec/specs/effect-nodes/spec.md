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
The rain colour SHALL be the environment's colour times 2.2, times 0.075·intensity + 0.075 (0.03·intensity + 0.03 with the bright setting), over 255, and its alpha (0.5·intensity + 0.5)·(|y| + 0.5).

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

### Requirement: Flame row ribbon
A lit flame row SHALL build its ribbon each frame as 16 sections of 8 vertices between the frame-interpolated pair of ends, each section's direction the interpolated lag delta (scaled by 0.05) normalised, with 192 indices and the black, black, white, white corner colours.

#### Scenario: The ribbon's size
- **WHEN** a lit row's ribbon is built
- **THEN** it has 128 vertices and 192 indices

### Requirement: Flame row section width
A flame row section's half width SHALL be 1.5 times the line over 16 divided by the length of its direction crossed with the unit vector from the camera, and its far side SHALL be moved by twice that plus the direction's length times the line's 0.2 share (taken in double and rounded once) times one less the absolute face-on weight.

#### Scenario: A section level with the camera
- **WHEN** a row 8 cm long whose lag lies 320 cm along +x is seen from 10 m along −z at mid-frame
- **THEN** its first section spans from −0.75 to 2.3 cm along x and its faces 0.75 cm either side

### Requirement: Flame row material
The flame row SHALL draw with the shipped `_FlameMat01` fragment stage over a 1 × 5 palette running from (0, 0, 0, 0) to (255, 230, 200, 64), its `Anim` uniform the row's texture offset, twice its negative and half of one less the fade.

#### Scenario: The palette ends
- **WHEN** the palette image is built
- **THEN** it is 1 by 5 texels with the first texel clear and the last (255, 230, 200, 64)

### Requirement: Portal import and dissolve ring
A portal marker loaded into a world whose flags carry `0x10000` SHALL be replaced by a fresh marker at its place keeping its render value, its 4-byte payload read and unused and a shorter payload refused; a dissolve burst SHALL write its seeded particles into the ring's slots in order up to the capacity and clear the system's `0x2` flag.

#### Scenario: A short payload
- **WHEN** a marker carrying the import flag has a 3-byte payload
- **THEN** no marker is made

### Requirement: Portal spray life
A portal spray particle SHALL start with the particle system's full lifetime, and its look index SHALL be written only under the 3-bit or 5-bit look flags.

#### Scenario: No look flag
- **WHEN** a spray without look flags spawns into a slot whose look is 7
- **THEN** the slot's life is the system's lifetime and its look stays 7

### Requirement: Electric template tone
The electric template selection SHALL take the strength (level·0.05 + scale) + (1 − level·0.05)·curve with the level term in double and a negative level counted as zero, clamped above at one with a not-a-number kept, and the intensity roll + level·0.4 in double.

#### Scenario: A clamped strength
- **WHEN** the level is 5 and the scale 2
- **THEN** the strength is 1

### Requirement: Electric template opacity
The opacity SHALL be 0.7 − strength·0.6 in double, blended as opacity·0.3 + owner·0.7 for a positive owner alpha; for level 5 on an owner whose state word is zero it SHALL be 0.4 − curve·0.375 with the product in single precision.

#### Scenario: The peak opacity rounds its product first
- **WHEN** the peak tone's curve sample is 0.84743375
- **THEN** its opacity is 0.08221235

### Requirement: Electric brief effect
The brief effect SHALL be chosen never at level 5, always below 3, at 3 when the engine generator's draw below 2 is 0 and above 3 when its draw below 8 is 0, with lifetime 0x16 against 0x44.

#### Scenario: A level-3 draw
- **WHEN** the generator's state is 0 at level 3
- **THEN** the effect is brief

### Requirement: Force dome hum
The Bubble's frame SHALL set its hum to (0.2, 0.75) standing, (x·0.2, 1 − x·0.25) with x = 4s(2 − 4s) growing under a share of 0.25 and ((2 − |1.5 − 3.5s|)·0.4, (1 − s)·0.75) shrinking, the 0.2 and 0.4 products in double, then add its glow times the phase gain (the share growing, one less it shrinking, else 1) once to the volume and 2.25 times to the pitch. The Barrier's frame SHALL use (1 − |0.5 − 1.5s|)·0.4 shrinking with the same double products and add four and nine times its hit glow.

#### Scenario: A shrinking Bubble
- **WHEN** a Bubble at half its share with a glow of 0.5 shrinks
- **THEN** its hum is (0.7 + 0.25, 0.375 + 0.5625)

### Requirement: Force dome GI lights
While a Barrier or a force Bubble lives it SHALL carry a GI light: the Barrier's `gi_dome` in its cap node's frame (the node's place and facing, the cap's apex the shell's +z pole) with the node's 85 × 85 × 25 cm box 75 cm ahead plus 60 cm; the Bubble's `gi_bubble` at its place, radius 5 past the dome's.

#### Scenario: The cap's apex
- **WHEN** a standing Barrier at (200, 0, 0) faces +x
- **THEN** its light maps the point 100 cm ahead to the shell's +z pole at 100

### Requirement: Barricade GI light
While a barricade lives it SHALL carry a `gi_cuboid_fn` light in the wall's frame at the node's place, its extent the node's half size as its last rising or falling frame wrote it less 60 across and 40 up; the body SHALL be the shipped program: the projected texture's ridge cubed, times 6 and the box falloff, less 0.01.

#### Scenario: A risen barricade's light
- **WHEN** a barricade has fully risen with nothing in its way
- **THEN** its light's extent is (240, 60, 5)

### Requirement: Force wave mesh and charge loop
A force wave's ribbon mesh SHALL allocate eight trail slots a chunk, 896 vertices and 1344 indices; its charge loop SHALL start silent at pitch 1 with the gain (base·0.8)·roll in double.

#### Scenario: The loop's gain
- **WHEN** the loop's base is 0.5 and its draw 0.5
- **THEN** its volume is 0 and its gain 0.2

### Requirement: Effect hazards
The live force blasts and waves SHALL carry a hazard holding their place and whether they were released, kept current after each tick, that answers a viewer with the blast's or the wave's assessment. A barricade SHALL carry a hazard holding its wall line while it rises or stands and drop it when it falls, and the sight SHALL offer a viewer the nearest barricade in sight.

#### Scenario: Out of sight
- **WHEN** a wave 13 m from a viewer is assessed
- **THEN** the viewer sees nothing

#### Scenario: The nearest wall
- **WHEN** a viewer has a barricade 6 m ahead and another 50 m ahead
- **THEN** the sight offers the nearer one, with flag 4 set

### Requirement: Tendril smoke program
The displacement tendril's smoke SHALL have the shipped `effectsmoke_f` body over the shared force ribbon vertex stage: three noise reads, the taper and a black output whose alpha is the smoke's, premultiplied.

#### Scenario: Smoke and ribbon differ
- **WHEN** a smoke material and a blast material are keyed
- **THEN** their keys differ and both blend premultiplied

### Requirement: Fire colour ramp
The fire unit's object conversion SHALL turn the record's ramp byte b into (20, 200) up to 0x46, ((b − 10)·2, (b + 10)·2) below 0x5A and ((2b − 100)·2, 200) from 0x5A, in wrapping byte arithmetic.

#### Scenario: A wrapping ramp
- **WHEN** the ramp byte is 0x7F
- **THEN** the pair is (52, 200)

### Requirement: Strikes light shape
The strikes' light shape SHALL be a `gi_gradline` with the strikes light's segment, gradient, point light, attenuation 10000 and colour, in the weapon node's box plus 60 cm.

#### Scenario: The point light's weight
- **WHEN** the strikes light's point carries a strength of 0.5
- **THEN** the shape's point light weight is 5

### Requirement: Rain pass matrices
Each rain pass SHALL upload as `ObjMat` the drop set's turn with every entry multiplied by the single-precision square root of the camera field times the single 1/70, placed at the drop box's centre and multiplied by the pass's sub-frame composite, and as `EyeVec` the camera less the box centre dotted with each scaled row.

#### Scenario: A field of 280
- **WHEN** the camera field is 280
- **THEN** the turn's entries are doubled and the object matrix's last row is the box centre with w = 1

### Requirement: Rain passes
The rain SHALL draw one pass per sub-frame with its turn two degrees about the first axis, followed, when the reflection level is above three, by a reflected pass at −2° about the first axis and 170° about the second.

#### Scenario: Reflection level three
- **WHEN** the reflection level is three
- **THEN** each sub-frame draws one pass

### Requirement: Vanish phase table
The displacement unit's phase table SHALL hold eight entries: (0, 0, −16, none), (0x1100, 0, 350, stage), (0x600, 350, 450, reveal), (0x300, 450, −16, show), (0, 500, −16, none), (0x200, 0, 200, stage), (0xe00, 200, 400, hide) and (0x1100, 400, −1, none).

#### Scenario: The table's ends
- **WHEN** the table is read
- **THEN** entries 0, 3 and 4 end at −16 and entry 7 at −1

### Requirement: Vanish phase advance
The frame callback SHALL run an entry's callback and advance the phase only when the elapsed time exceeds its end compared unsigned; the phased body's hook SHALL hide and let go on the −1 entry and otherwise move to phase 4 and show.

#### Scenario: The reveal
- **WHEN** the elapsed time passes 450 ms in phase 2
- **THEN** every body is shown 1 with the show call 0x20, the items are shown 1 and the phase becomes 3

#### Scenario: The last entry
- **WHEN** the phase is 7 at any elapsed time
- **THEN** the frame callback does not advance it

### Requirement: Force geometry pass uploads
A force geometry with more than one pass SHALL upload pass 0's object matrix, set pass 0's viewport and draw, then for each later pass set its viewport, upload its matrix and draw; with a single pass it SHALL draw nothing on that path.

#### Scenario: One pass
- **WHEN** the view has a single pass
- **THEN** no object matrix is uploaded by the force geometry's frame
