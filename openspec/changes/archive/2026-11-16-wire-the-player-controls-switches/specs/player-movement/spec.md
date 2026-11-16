## MODIFIED Requirements

### Requirement: When the reach is measured
The reach SHALL be measured each frame: outside combat mode while the behaviour's walk latch does not hold a mouse-driven walk to the cursor, and in combat mode only with Ctrl held or something held, whatever the right button does. A drag in hand SHALL be within reach. The player down, collapsing or dead SHALL let go of everything and be out of reach. Outside combat mode the cursor SHALL show the pointer within reach and the cross beyond it.

#### Scenario: A refused press and the cursor
- **WHEN** a press is refused outside combat mode over a point within reach while walking to the cursor
- **THEN** the cursor shows the cross

### Requirement: Drag lets go
Each frame of a drag the grabbed point SHALL be let go once its squared distance from the player's hip, compared in double precision, is over 108 900 cm² while the drag keeps its reach limit, when the held body is a character that lives, or when the left button is released. While the controls are off, blocked or second-blocked the frame SHALL leave the drag waiting for its release.

#### Scenario: Letting go
- **WHEN** the grabbed point is 330 cm, 330.1 cm and (200, 200, 200) cm from the hip
- **THEN** 330 cm holds and the other two let go

#### Scenario: A blocked frame
- **WHEN** the controls are blocked with a drag in hand, then released
- **THEN** the drag waits while blocked and is run again after the release

### Requirement: Drag follows the cursor
Within reach, the drag's anchor SHALL be where the cursor's ray meets the level plane at the grab's height, kept within 200 cm of the hip across the ground (and then at the grab's height) while the drag keeps its reach limit, and the body SHALL be woken to 256; out of reach the anchor SHALL stay on the grabbed point.

#### Scenario: Following
- **WHEN** the cursor's ray falls straight onto the 30 cm plane 1.5 m away, then 5 m away along (3, 4), then 5 m away without the limit
- **THEN** the anchor is at 1.5 m, then pulled to (1.2, 0.3, 1.6) m, then at (3, 0.3, 4) m

### Requirement: Entering combat mode
Entering combat mode SHALL let go of what the hand holds and clear the request's backpedal, keys' heading and turn-about bits; a lifted item SHALL go back to the frames its lift saved. Tab SHALL toggle a living player's combat mode in the running game.

#### Scenario: Letting go
- **WHEN** combat mode is entered with a drag in hand and the request carrying move, backpedal, keys' heading, turn-about and run, and Tab is pressed by a living and by a dead player
- **THEN** the drag is let go, the request keeps move and run, the living player's combat mode turns on and the dead one's does not

#### Scenario: A lifted item
- **WHEN** Tab is pressed while an item lifted at one place hangs on the cursor at another
- **THEN** the hand is empty and the item is back where the lift took it

## ADDED Requirements

### Requirement: Walk latch gates the reach
The walk latch SHALL be set on a move's first frame and, after the release, held until the stride reaches a double support; a walk the keys drive SHALL measure the reach, and the right button alone SHALL not stop it.

#### Scenario: The latch, not the button
- **WHEN** the latch holds a walk to the cursor with the right button up, then the latch is clear with the right button down and nothing under the cursor
- **THEN** the reach is not measured the first time and is out of reach the second

### Requirement: Drag reach limit
The drag SHALL keep its reach limit from the controls' creation; either block pair's block SHALL clear it and its release SHALL set it again.

#### Scenario: Block and release
- **WHEN** the controls are blocked, then released
- **THEN** the limit is off while blocked and on again after

### Requirement: Spirit form holds the controls
The spirit form's start SHALL hold the controls with only the powers' hotkey permission, keep the hand's left button off the world, close every game screen with the shown power bar and end the dialogue; its way home SHALL give all five permissions and the button back.

#### Scenario: Spirit form and home
- **WHEN** the spirit form starts behind a free-running simulation and later takes the player home
- **THEN** the frame's controls hold mask 4 and the kept button, then mask 0x1f and the button free

### Requirement: Keys in spirit form
While the spirit form holds the controls the inventory key SHALL ask for nothing; the Ctrl interaction SHALL do nothing and a pack drag over the world SHALL not be claimed. The shop's trade mode SHALL keep the inventory key to the collapsing player too.

#### Scenario: The inventory key
- **WHEN** I is pressed in spirit form, then after the way home
- **THEN** no inventory request, then one

### Requirement: Item cursor turn keeps its frame square
Each turn of a lifted item SHALL make the held frame orthonormal again: the second axis normalised, the first made orthogonal to it and normalised, the third their cross normalised, each normalisation the engine's approximate one.

#### Scenario: A sheared frame turned
- **WHEN** a lift whose frame drifted to a sheared one is turned by the forward key for a frame
- **THEN** its axes are unit and square within the approximation

### Requirement: Drop scripts
A lifted item the press took off a lock SHALL, when its release leaves that lock (anything but a relock within 5 cm² of the spot), run the scripts of every object closer than 5 cm² to the spot; a put-back release SHALL lock the item again there.

#### Scenario: The crystal
- **WHEN** a crystal locked on a device's slot is lifted and put down elsewhere, and again put back on the slot's lock
- **THEN** the device powers down the first time and not the second

### Requirement: Power hold
A bar cast answered "not now" SHALL hold its slot in the player's controls for 500 ms; while the controls are on the frame SHALL retry it every frame, a "not now" SHALL renew the hold, any other answer SHALL let it go, and an expired hold SHALL ask once for the picked power.

#### Scenario: Switching weapons
- **WHEN** a shock is cast while the body switches its weapon, and the switch ends two frames later
- **THEN** the slot is held and retried, then the shock starts and the hold lets go

### Requirement: GUI drag ends
The death, the knock-out, the level exit and a weapon coming loose from the player's hand SHALL release a held left button for the next frame's input, ending a held drag as a release does.

#### Scenario: Death with the button down
- **WHEN** the player dies with the left button held
- **THEN** the next frame sees the button up

### Requirement: Object use starts its tracks
A double click's use of an unmarked kind 0 object SHALL start the keyframe tracks of its node and the nodes under it before its use sound, and the object SHALL stay in the scene.

#### Scenario: The secret switch
- **WHEN** a kind 0 subtype 1 object is double clicked
- **THEN** its tracks start, its sound plays, its target's signal is queued 2100 ms on and it stays

### Requirement: Idle double click
With the controls off a double click SHALL be the hand's own hook: a class 0 object's database use SHALL run at once, another usable class SHALL lift the use cursor, and a container SHALL open.

#### Scenario: Controls off
- **WHEN** a divination deck, a usable class 1 item and a chest are double clicked with the controls off
- **THEN** the reading opens with no tracks or sound, the use cursor lifts and the chest's popup is asked for

### Requirement: Lift warps the pointer
The press's lift SHALL move the pointer onto the lifted node's screen point with the warp hold.

#### Scenario: Warp request
- **WHEN** two warp requests arrive in a frame
- **THEN** the pointer lands on the last one and the hold starts
