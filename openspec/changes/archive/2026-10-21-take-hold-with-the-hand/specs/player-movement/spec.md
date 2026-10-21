# Spec Delta

## MODIFIED Requirements

### Requirement: Cursor pick
A click SHALL name a free hinge as a door, a dead character as a corpse and a living one as a person. A loose prop whose mobility is under 0.08 SHALL be ignored unless it is tagged bulky, and a prop needing strength SHALL be ignored when strength is under 0.9. A usable item on a prop SHALL stay usable, and a usable item with no body SHALL be nothing. A point farther than 70 cm SHALL be turned toward first.

#### Scenario: A door, a light prop, and the reach
- **WHEN** a free hinge, a blocked hinge, a prop of mobility 0.07, a prop of mobility 0.08, a bulky prop of mobility 0.07, a usable prop, that usable prop with its body removed, a point 70 cm away and a point 70.1 cm away are read, and the reach is tried at 245 cm high and 300 cm away, then 0.1 cm farther, then as a door at 350 cm
- **THEN** the names are a door, nothing, nothing, a loose prop, a loose prop, usable, nothing; 70 cm does not turn and 70.1 cm does; the first reach holds, the farther one does not, and the door does

## ADDED Requirements

### Requirement: Reach under the cursor
A point under the cursor SHALL be within reach when the point the pick hit is at most 245 cm high in the world and its offset from the player's hip is at most 300 cm up or down and 300 cm across the ground; a door SHALL be measured from its own place with 300 cm of height and 350 cm across. A comparison with a NaN SHALL pass.

#### Scenario: The limits
- **WHEN** the reach is tried with the hit 245 cm high and 300 cm away, 245.1 cm high, 300.1 cm away, 300.1 cm below, 500 cm high right at the hip, as a door 300 cm high and 350 cm away, and with NaN values
- **THEN** the first holds, the next four do not, the door holds and the NaN values pass

### Requirement: When the reach is measured
The reach SHALL be measured each frame while the right button is not walking to the cursor, in combat mode only with Ctrl held or something held. A drag in hand SHALL be within reach. The player down, collapsing or dead SHALL let go of everything and be out of reach. Outside combat mode the cursor SHALL show the pointer within reach and the cross beyond it.

#### Scenario: A refused press and the cursor
- **WHEN** a press is refused outside combat mode over a point within reach while walking to the cursor
- **THEN** the cursor shows the cross

### Requirement: Left press
A left press SHALL be ignored while combat mode keeps the button for attacks (unless Ctrl is held or something is held), while the point under the cursor is out of reach, while the player is down, or when the pick hits nothing. Each press SHALL start from nothing held.

#### Scenario: A busy hand
- **WHEN** a corpse is pressed in combat mode without Ctrl, out of reach, and with the player down
- **THEN** each press is ignored

### Requirement: What a press takes
A prop or item the hand can move SHALL, unless the player locomotes, begin the object's hold (the standing quarter turns wait) and with Ctrl be dragged by the part the pick hit at the hit point, strength 1, in the tick's first two passes; without Ctrl it SHALL hold until the release, which ends the hold. A corpse SHALL be dragged by the limb the pick hit, strength 0.5, every pass. Anything else but a door, a wheel or a sim object SHALL be refused until the release.

#### Scenario: What a press starts
- **WHEN** a loose prop is pressed with Ctrl, without Ctrl and while locomoting, a corpse with and without Ctrl, a person, the level and a crank
- **THEN** the prop is dragged at strength 1 for passes 0 and 1 with the object held, lifted, and ignored; the corpse is dragged at 0.5 in every pass; the person, the level and the crank are refused

### Requirement: A refused press
A refused press SHALL show the cross outside combat mode and SHALL leave the movement request as it was until the release; the lift SHALL leave the request as it was too.

#### Scenario: The request is kept
- **WHEN** the hand holds a refusal, then a lift, then a drag
- **THEN** the first two keep the request and only the refusal shows the cross

### Requirement: Drag lets go
Each frame of a drag the grabbed point SHALL be let go once its squared distance from the player's hip, compared in double precision, is over 108 900 cm², when the held body is a character that lives, or when the left button is released.

#### Scenario: Letting go
- **WHEN** the grabbed point is 330 cm, 330.1 cm and (200, 200, 200) cm from the hip
- **THEN** 330 cm holds and the other two let go

### Requirement: Drag follows the cursor
Within reach, the drag's anchor SHALL be where the cursor's ray meets the level plane at the grab's height, kept within 200 cm of the hip across the ground (and then at the grab's height), and the body SHALL be woken to 256; out of reach the anchor SHALL stay on the grabbed point.

#### Scenario: Following
- **WHEN** the cursor's ray falls straight onto the 30 cm plane 1.5 m away, then 5 m away along (3, 4)
- **THEN** the anchor is at 1.5 m, then pulled to (1.2, 0.3, 1.6) m

### Requirement: Pick under the cursor
The pick SHALL look 40 m along the cursor's ray at the level and the props' meshes, naming a prop's body and part by its mesh, and at the characters' collision spheres, the nearest hit first.

#### Scenario: The nearest character
- **WHEN** a dead character's sphere 3 m ahead and a living one's 6 m ahead are on the ray, with nothing fixed
- **THEN** the pick hits the dead one at 2.8 m and names a corpse, and a ray beside them hits nothing
