## MODIFIED Requirements

### Requirement: Feral thorax hunch
The pinned part's posed placement SHALL lower its forward axis by the thorax hunch after the torso twist when the character's feral level, in double, is over 0.1, then rebuild the basis from that forward, the up axis and their cross; no other part's placement changes. The hunch rises 0.01 a tick in double precision to 0.3 on its feet and falls 0.01 to 0 off them. A balanced attack tick (an attack's clip playing, not off balance, shocked or getting up) SHALL take a further 0.02 off.

#### Scenario: Standing thorax
- **WHEN** a pinned part's placement is unturned and its hunch is 0.3, with feral level 1
- **THEN** its forward axis drops below −0.25, its place and the other parts' placements stay

#### Scenario: Not feral
- **WHEN** the same part's feral level is 0, or 0.05
- **THEN** its placement stays the pose's

#### Scenario: The attack's ease
- **WHEN** a body with hunch 0.3 attacks on balance, and then attacks off balance, shocked or getting up, or stands normally
- **THEN** the hunch is 0.28 after the balanced attack tick and stays 0.3 otherwise

## ADDED Requirements

### Requirement: Placement key pick
The placement's sampler SHALL pick its keys at clock t with lead-in b as the living sampler does but without an ease, a clamp or a guard between the keys: up to the first key by t / (first + b) (0 when the first key's time is at most 1.19209e-7), past the last key looping by (t − b − last) / first; the bones blend from the capture while the target key is the capture's, otherwise the clip rebases. For Walk, Run and Backpedal with muscles on its root SHALL come from the source key.

#### Scenario: Lead-in without ease
- **WHEN** a clip with keys at 10, 20 and 40 frames is sampled at clock 9 with a lead-in of 5 aiming at the capture's key
- **THEN** the bones blend from the capture to key 0 by 0.6

### Requirement: Queued clip switch
The living sampler SHALL switch to a queued clip only on a tick whose target key differs from the last tick's: it then clears the feet's contact bits, captures the current targets, takes that key as the capture's and does not sample.

#### Scenario: Same target key
- **WHEN** a clip is queued and this tick's target key is last tick's
- **THEN** the playing clip is sampled and the queued one waits
