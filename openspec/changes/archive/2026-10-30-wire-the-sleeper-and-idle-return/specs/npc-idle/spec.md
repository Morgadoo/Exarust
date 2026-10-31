# Spec Delta

## MODIFIED Requirements

### Requirement: Sleep
A sleeper SHALL become dormant and set the dormant flag without clearing the others. Its tone and tone target SHALL become 0, it SHALL be a statue, and its gait SHALL be 2. The tick installed afterwards SHALL set the dormant flag again, without clearing the others, and make the condition dormant again, and SHALL change nothing else. A second sleep SHALL keep that flag and the dormant condition.

#### Scenario: A walking body lies dormant
- **WHEN** a walker whose condition is walking, whose flags are the shock stagger, whose tone is 1 and whose gait is 0 sleeps, its tick runs, its condition is then rewritten to down and its flags to waking alone and its tick runs again, and it sleeps again
- **THEN** the condition is dormant, the flags are the stagger and the dormant flag, the tone and the tone target are 0, it is a statue with gait 2, the first tick leaves all of that as it was, the second puts back the dormant condition and the flags waking and dormant while the statue and gait stay, and the second sleep keeps it

## ADDED Requirements

### Requirement: Sleep in play
A level character whose applied roles choose the sleep activity SHALL lie dormant on its next AI tick: condition dormant, the dormant flag joined to its flags, tone 0, and the body's statue rest set. While the sleep is its current state and it is not fighting, each AI tick SHALL make the condition dormant and set the dormant flag again, and SHALL neither run the idle head nor turn the body. Rating the player SHALL NOT wake it.

#### Scenario: The shipped sleeper lies down
- **WHEN** a character holding Activity Sleep is placed walking with the shock stagger, its condition is later rewritten to down, and its rating of the player rises to a concern of 1.5
- **THEN** after its first AI tick its condition is dormant and its flags the stagger and the dormant flag, its statue rest is set, the next AI tick makes the condition dormant again, and it keeps the sleep role

### Requirement: Sleeper wake in play
A character whose state flags are exactly the dormant flag and whose target pass classifies anyone above a notice SHALL drop role 44, apply its roles again on the next AI tick, and be knocked out with no delay (condition down, flags dormant and waking, the wake clock now, its statue rest cleared), for the body tick's wake to raise it. Other flags SHALL NOT be knocked out by the pass.

#### Scenario: Seen and knocked out
- **WHEN** a sleeper with a hostile contact runs its target pass, and a knocked-out body with flags 0x30 and the same contact runs its own
- **THEN** the sleeper loses role 44, is down with flags 0x30, a wake clock at 0 ms, no longer sleeps and is engaged, its next AI tick applies the roam, and the knocked-out body gets no new knock-out and keeps its flags

### Requirement: Chase idle return
A fighter whose chase route leaves (not following a route, no fresh foe) SHALL leave combat through the idle return: its look, heading and eased offset SHALL take the body's look yaw, its idle state's step SHALL restart at the first wait with its deadline kept, its idle state SHALL become the current state (the sleep tick again for a sleeper) and the body's in-combat mark SHALL clear.

#### Scenario: Back to the roam
- **WHEN** a roaming brain in its home walk, marked in combat, returns to idle, and then a sleeping brain does
- **THEN** the roam is in its first wait with the mark cleared and not sleeping, and the sleeper's sleep tick is the current state again

### Requirement: Investigation in play
The wait's head every tick, and a roam's or patrol's in its two waits, SHALL open with the investigation for a brain in the scene that follows no route, sampling the level pathfinder's disturbance at its 600 cm cell. The shipped disturbance SHALL be 0 everywhere, so the investigation SHALL neither stamp nor walk; it SHALL keep the cell it sampled.

#### Scenario: Never a noise
- **WHEN** a brain samples three times at the origin, at (−601, 0, 1199) cm and at (5, 5, 5) cm
- **THEN** every sample does nothing, the kept cell follows the body, and the second cell is (−2, 1)

### Requirement: The idle driver stays uninstalled
No activity, idle activity or level character SHALL install the room-walking idle driver or its go-to-object state: in the original they are installed only on the player body's behaviour behind a flag that is never set, and by the driver itself.

#### Scenario: Activities
- **WHEN** roles choose activities 0 to 7
- **THEN** the states are the roam, wait, sleep, follow, sentry and patrol (the others wait), and none walks rooms to use objects
