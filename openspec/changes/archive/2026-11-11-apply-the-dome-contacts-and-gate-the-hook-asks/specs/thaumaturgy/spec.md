## ADDED Requirements

### Requirement: Player hook asks stop while staggered or suspended
The player's controller SHALL ask no cast hook in a tick while the body's shock-stagger state flag is set, while the player's behaviour is suspended (the controls blocked or second-blocked), or while the body collapses or is dead.

#### Scenario: A shock-staggered caster
- **WHEN** a shock-staggered player holds the left button with a strikes hook set
- **THEN** the hook is not asked and the cast goes on

### Requirement: Dome contact per pushed part
A bubble or barrier SHALL run its contact once for each part of a character it pushed, measuring from the character's chest point. A held body with no holder SHALL take no contact.

#### Scenario: Two parts pushed
- **WHEN** a bubble pushes two limbs of another character out in one tick
- **THEN** that character's instability counter rises by two

### Requirement: Dome contact knock
A contact on another character's limb within the dome's radius plus 20 times its size SHALL shove it 40 cm away, raise its instability counter by one, lower its energy above 0.25 by 0.002 for the bubble or 0.001 for the barrier within 100 cm and by `(d − 100)·0.0001` more beyond, and make its AI drop the route it follows.

#### Scenario: A duellist against the bubble
- **WHEN** a duellist's chest is 120 cm from a standing bubble's centre and a limb is pushed out
- **THEN** it is shoved, its counter rises by one, its energy falls by 0.002 and its AI plans its route again

### Requirement: Dome contact on a held weapon
A pushed part of a held body SHALL only break its holder's attack and play the hit sound when it is the weapon's strike chain; the attack breaks even within the sound's quiet time.

#### Scenario: A sword in the bubble
- **WHEN** a bubble pushes an attacking duellist's sword
- **THEN** the duellist's attack is interrupted

### Requirement: Barricade shove precision
The barricade's push SHALL be computed in double and narrowed once, the tenth from behind (or a NaN depth) in double, the energy loss subtracted in double, and its hit sounds' draws scaled in double; each knock SHALL also make the character's AI drop its route.

#### Scenario: A limb behind the wall
- **WHEN** a sphere is pushed from behind the wall
- **THEN** its push is the front push times 0.1 in double

### Requirement: Player domes on the standing point
The player's bar casts SHALL place the bubble and barrier from the body's standing point (its transform's place), as the duellists' casts do.

#### Scenario: A player's bubble
- **WHEN** the player standing on the arena floor casts the bubble
- **THEN** its centre is 100 cm above the floor

### Requirement: Control Mind sweeps the power tree
Control Mind's handover and hand-back SHALL first sweep the power tree: a shown tree closes as its close does, else a shown bar falls.

#### Scenario: The bar during a handover
- **WHEN** the player's bar is shown as Control Mind hands the target over
- **THEN** the bar falls

### Requirement: Control Mind behind the bridge
Control Mind's handover, the hand-back's transition and Spirit Form's end SHALL run on the frame's world when the simulation runs behind the bridge.

#### Scenario: A handover behind the bridge
- **WHEN** Control Mind binds its target with the simulation behind the bridge
- **THEN** the player takes the target's body and no command fails
