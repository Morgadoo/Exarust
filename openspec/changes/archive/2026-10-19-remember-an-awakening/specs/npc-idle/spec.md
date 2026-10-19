# Spec Delta

## ADDED Requirements

### Requirement: Witness memory
On the tick that drops the dormant flag, each seeing person SHALL update memory, and nobody else. A later tick SHALL leave it. A short count SHALL add one and keep counting. Meeting the need SHALL stamp the seconds plus the delay and set the high bit. A cooldown, a future last use, and never-again SHALL stay. Inside its window an activated stamp SHALL refresh to now, and at the window re-arm with the delay. A future stamp SHALL report updated and stay. Held-off SHALL leave an activated word.

#### Scenario: Who can see stores a word
- **WHEN** a knocked-out body is ticked at 1501 ms at second 100 with a seeing person who needs two hits and whose word is 0, a seeing person already counting 1 with a 2 second delay, a seeing person already counting bit 1 who is witnessed as bit 2, a seeing person whose last use is second 1000, a person who cannot see, and a non-person who can, then ticked at 1502 ms, and a knocked-out body is ticked at second 1009 and another at second 1010 with a ten-second cooldown and a last use of 1000 that needs two hits, and the shipped update is asked for a never-again nibble, an activated stamp one second inside a ten-second window, that stamp at the window with a 2 second delay, a future activated stamp, the held-off flag, and a never-again window at the last second with a 2 second delay
- **THEN** the short count is 1 and still counting, the met need stamps 102 with the high bit, the second bit stamps 100 with the high bit, the last use of 1000 stays, the person who cannot see and the non-person stay, the later tick leaves the count, second 1009 stays at 1000, second 1010 counts 1, never-again stays, the inside stamp refreshes to 109 with the high bit, the window re-arms at 112, the future stamp stays and reports updated, held-off stays, and the last second refreshes to itself with the high bit
