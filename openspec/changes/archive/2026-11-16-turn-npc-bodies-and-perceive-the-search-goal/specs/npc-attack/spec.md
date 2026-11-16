## ADDED Requirements

### Requirement: The tick's look
The combat tick's end SHALL write the body's look direction from the AI's heading and copy it into the look smoothing. A charge SHALL write the look direction toward the target point on the ground and take the AI's heading from it, leaving the look smoothing.

#### Scenario: A stab and a charge
- **WHEN** a pressing fighter's tick ends while a stab plays, and a fighter running faster than 3 cm a tick charges
- **THEN** the first writes both directions from its heading; the second writes only the look direction, toward the target point, and its heading is that direction's yaw

### Requirement: The stab's head tracking
While a stab (motion 11) plays, the blow's sweep SHALL run the AI's head tracking: its yaw and pitch step at most π/100 toward the AI's estimate from the eye by the shoulder, the yaw held within 45° of the body's facing, the stab's aim point 250 cm along the tracked look; the look smoothing and look direction take it before the tick's end writes them again.

#### Scenario: A head half a radian off
- **WHEN** a duellist's stab plays with its head yaw at −0.5 and its estimate straight ahead
- **THEN** the yaw steps to −0.5 + π/100 and the aim point lies 250 cm along the tracked look from the eye

### Requirement: The stab start's head
A stab start SHALL set the AI's head yaw to the yaw of the body's look smoothing (the AI's aim without body directions) and its pitch to 0.

#### Scenario: A stab begins
- **WHEN** a duellist whose look smoothing lies along 0.3 starts a stab
- **THEN** its head starts at yaw 0.3, level, with no aim point

### Requirement: The chase's grip mode
While its route is followed the chase SHALL set the body's grip mode 1 every tick, ahead of a motion start's and the locomotion's modes in the same tick; a mode it set SHALL stay until another is set.

#### Scenario: A chase walking its route
- **WHEN** a stuck fighter's chase has planned its route and follows it
- **THEN** its body's grip mode asked is 1, and a held item with no clip start or locomotion that tick takes mode 1
