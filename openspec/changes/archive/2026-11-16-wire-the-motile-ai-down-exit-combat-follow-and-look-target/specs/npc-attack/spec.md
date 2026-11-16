## ADDED Requirements

### Requirement: The follow check runs in combat
For a fighter whose activity is the follow, the combat tick SHALL first run the follow check while the body does not step; the leader SHALL be the player, as the game publishes it each body tick (its place, its facing, its request's run and sprint bits). A fighter that does not follow SHALL never be called back.

#### Scenario: Not a follower
- **WHEN** a fighter without the follow activity fights with a leader published
- **THEN** its band stays 400 cm and it walks no route

### Requirement: The follow check's walk
Inside the band a follower walking to its leader SHALL soft-stop and its mode word clear; outside it the walk's urgency SHALL be set, a follower walking to its leader SHALL let it pass the walk's in-the-way test, and a plan SHALL be forced to the leader's ground point 20 cm up (stop 40, size class 1, the follow's half-height end test), which on success follows the leader (mode 4) from where it stands. A follower walking to its leader SHALL end its combat tick there.

#### Scenario: Called back
- **WHEN** a fighting follower stands 1500 cm from its leader, then walks to it, then the leader comes within 250 cm
- **THEN** it plans the walk at urgency 2 with its tick going on, then its tick ends at the follow, then its walk stops with the mode word 0
