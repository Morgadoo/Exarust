## MODIFIED Requirements

### Requirement: Recovery step target
While a step is wanted the target SHALL be the chest plus a lead from the whole 3-D fast velocity v: 15·scale·k ticks of it when |v|² is at most (2·scale)² cm², else (30·scale/|v|)·k of it, k = (2 − S)(2 − E⁴·S), keeping only its horizontal part and the height it is measured from. A body not stepping SHALL first reset its target to the standing position.

#### Scenario: A slow chest
- **WHEN** support and energy and scale are 1 and the fast chest velocity is 1 cm a tick
- **THEN** the lead is 15 cm along that velocity

#### Scenario: A fast chest
- **WHEN** the same velocity is 5 cm a tick
- **THEN** the lead is 30 cm along it

#### Scenario: A falling chest
- **WHEN** the velocity is 1.5 cm a tick across and 1.5 down
- **THEN** the lead is 1.5·30/√4.5 cm across

### Requirement: Knockback step target
A knockback (or long) step SHALL be re-targeted by the trigger as any step while the chest is past it, its target kept otherwise, until balance is under 0.2, it gets up or the body locomotes.

#### Scenario: Chest past the trigger
- **WHEN** a knockback step's chest is 1 m out
- **THEN** its target moves under the chest and its stance origin is clamped to 88 cm

#### Scenario: Balance gone
- **WHEN** that step's balance falls under 0.2
- **THEN** the step is cleared

### Requirement: Stepping episode
A step SHALL end after 100 ms of its start clock with no foot anchored, after the chest has stayed within 20·scale cm of the target for more than 32 ticks, after 200 ms while the chest is farther and closing under 0.05 cm a tick, or after more than one landing. Ending SHALL plant the free foot nearest its spot plus the chest and 5 ticks of fast velocity and move the standing position onto it.

#### Scenario: The four ends
- **WHEN** 101 ms have passed with no foot anchored, or the chest has been near for 33 ticks, or 201 ms have passed with the chest 21 cm away and closing at 0.04 cm a tick, or the step has two landings
- **THEN** the step ends

#### Scenario: Still closing
- **WHEN** 50 ms have passed, a foot is anchored, the chest is 50 cm from the target and still closing, and there has been no landing
- **THEN** the step does not end

## ADDED Requirements

### Requirement: Trigger target taken
A wanted target more than 10·scale cm (squared) from the point it is measured from SHALL be taken and SHALL restart the episode's landing count; a body that does not locomote SHALL then step, a balance step when it was not stepping. A locomoting body SHALL still write the out-of-balance bit, the target and the count, and start no step.

#### Scenario: Walking past the trigger
- **WHEN** a walking body's chest is 1 m past the trigger
- **THEN** its target is under the chest at the standing height and it is not stepping

### Requirement: Start clock
The episode checks and the other foot's guard SHALL read the start clock the footwork, AI, queued and knockback starts write. A balance step SHALL write no start clock, near-target count or stand-still stamp; no start SHALL write the near-target count; a knockback and a collapse stagger SHALL restart the landing count.

#### Scenario: A balance step
- **WHEN** a balance step starts with an old start clock and near count
- **THEN** both are kept

### Requirement: Slow-turn hold
With the body's slow-turn word set (exploring, an AI idle state, a resting statue) the landing pass SHALL keep a reached foot's step until the later stand-still stamp is more than 100 ms old, and the other foot's landing outside an episode SHALL set that stamp 300 ms ahead. The reached foot's change stamp SHALL be written either way.

#### Scenario: Held then released
- **WHEN** the word is set and the stamp is 100 ms old, then 101 ms
- **THEN** the foot keeps its step, then lands

### Requirement: Footwork reach reference
A foot score's reach term SHALL be measured from the pelvis eased a quarter of the way toward the stance origin (a + (b − a)·0.25), and the scores and the anchor's clamp SHALL read the stored reach whatever the crouch.

#### Scenario: A pelvis off the frame
- **WHEN** the pelvis stands 40 cm ahead of the frame with the stance origin on the frame
- **THEN** the reach is measured from 30 cm ahead

### Requirement: Other foot lands
When a foot is chosen and the other foot is anchored, the other foot SHALL land where it is; with the episode's start clock more than 200 ms old and the chest strictly within 20·scale cm of the target, only the stepping byte SHALL clear: no step end, no foot planted by it.

#### Scenario: Near the target
- **WHEN** the clock is 250 ms old and the squared distance is 399 cm²
- **THEN** stepping clears and the standing position stays
