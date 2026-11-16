## MODIFIED Requirements

### Requirement: The staging freeze's span
A staging freeze set under a transition screen SHALL hold until the screen's close releases it, however many ticks that takes. A staging freeze set with no screen running SHALL hold for the screen's 500 ms hold and its 600 ms fade back in, counted as 92 body ticks from the first tick after it is set, and SHALL clear on the 93rd. Setting it again SHALL start it again. Frames that run no body tick SHALL not count.

#### Scenario: The release
- **WHEN** the freeze is set and the brains' chain runs 92 body ticks, one frame without a tick, and one more tick
- **THEN** it holds through the 92 ticks and the frame, and the 93rd tick runs the states

#### Scenario: A held freeze
- **WHEN** a freeze is held by a screen and three times 92 ticks run, then the screen's release comes
- **THEN** every tick is frozen and the release clears it
