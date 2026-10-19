# Spec Delta

## ADDED Requirements

### Requirement: Striker recoil
A recoil SHALL set strength and the stall rate to 0.5 and SHALL clear the released bit. A class word of 0x90 with the continuation bit, or with a clock whose widening exceeds 0.4 and a counter, SHALL stop there. Any other swing SHALL set stalling, cleaving and recoiled, end at the truncated widened drive times 0.1 times 400 plus now plus 200, use blend 40, and set both arm stiffnesses to 0.2.

#### Scenario: A blocked swing recoils unless two weapons hold it
- **WHEN** a released swing with the window bit and drive 1 is recoiled, drive 0.013 is recoiled, a class word of 0x90 with the continuation bit is recoiled, clock 0.4 with a counter is recoiled, clock 0.4 without a counter is recoiled, and the next lower clock with a counter is recoiled
- **THEN** drive 1 has strength and stall bits 0x3f000000, flags 0x27, end 1240, blend 40 and stiffness bits 0x3e4ccccd, drive 0.013 ends at 1200, the continuation and clock 0.4 with a counter leave no end, and clock 0.4 without a counter and the next lower clock end at 1240

### Requirement: Follow-through
A class word of 0x90 continuing, or a clock widened past 0.4 with a counter, SHALL write nothing. The scale SHALL be gain times recoil times speed times 0.15, floored at 0. A body zone at or under the epsilon SHALL set strength 0, else the square root times the scale. An edge SHALL use one minus the zone times the scale plus the zone. Strength SHALL cap at 1. Stall SHALL be one minus strength times 0.05, widened. Flags SHALL gain 6. The end SHALL be the truncated scale times 400 plus now.

#### Scenario: A wound slows the swing and a continuation does not
- **WHEN** gain, recoil and speed 1 meet zone 0.25, zone 0, the epsilon, an edge at 0.25, gain 100 at zone 1, gain -1, and speed 0.009 at zone 0, and a class word of 0x90 with the continuation bit is followed through
- **THEN** zone 0.25 has strength bits 0x3d99999a, stall bits 0x3d3d70a4, flags 0x7 and end 1060, zone 0 and the epsilon have strength 0, the edge has strength bits 0x3eb9999a, gain 100 has strength 1 and end 7000, gain -1 and speed 0.009 end at 1000, and the continuation writes nothing
