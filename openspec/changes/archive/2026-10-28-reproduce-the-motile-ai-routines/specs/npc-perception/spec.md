# Spec Delta

## ADDED Requirements

### Requirement: The AI tick's room cell and alert
Every frame each brain SHALL link itself into the 600 cm room cell of its body: the floored position over 600 cm on each ground axis, offset by half the grid. Off the grid its link SHALL stay unchanged. The group alert SHALL decay one step every 10 s, down to 1.

#### Scenario: Off the grid
- **WHEN** a brain in a 4 × 4 grid moves 100 m along x
- **THEN** it stays linked in its old cell

### Requirement: The idle turn
While not engaged, a brain's model SHALL turn to the out-of-combat eased yaw. The yaw starts at the body's facing and eases toward the look plus a sway by `0.09 − 0.06·encumbrance`. The sway is `u·3.6 − 1.8`, redrawn every 4000 ms plus up to 4000 ms.

#### Scenario: Easing toward the look
- **WHEN** an idle brain faces yaw 0 and looks along π/2 with no sway and no encumbrance
- **THEN** its model turns to 0.09·π/2 that frame

### Requirement: The ally-class sense range and fellows
The ally-class sense SHALL raise a concern between 1 and 3 to 3. It SHALL track a character within 1200 cm, or 1800 cm while tracked, ignoring height differences under 150 cm. A fellow AI character SHALL get attitude 100 and no concern.

#### Scenario: A fellow AI
- **WHEN** the observed character's race has the AI flag
- **THEN** its attitude is 100, its concern 0, and the sense returns −1

### Requirement: The ally-class threat
With a concern of at least 1, the ally sense SHALL take the largest threat to an ally contact. That threat is the contact's weight `(view·0.75 + 0.25)·closeness²` times the observed state's factor (4 attacking, 0.25 poised or 0.1, halved when down) times 1.2. When that threat is above the concern, the concern target SHALL become 0.3 of it.

#### Scenario: An attacker near an ally
- **WHEN** the observed character attacks, the concern is 1.5 and an ally contact has closeness 0.9 and view 1
- **THEN** the concern becomes 3 and the concern target 0.3·(0.81·4·1.2)
