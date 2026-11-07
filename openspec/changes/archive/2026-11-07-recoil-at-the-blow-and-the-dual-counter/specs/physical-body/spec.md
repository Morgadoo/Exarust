## MODIFIED Requirements

### Requirement: Striker recoil
A recoil SHALL set strength and the stall rate to 0.5 and SHALL clear the released bit. A class word of 0x90 with the continuation bit, or with a clock whose widening exceeds 0.4 and a counter, SHALL stop there. Any other swing SHALL set stalling, cleaving and recoiled, end at the widened drive times 0.1 times 400 plus now plus 200, switch at once to the stance by energy (blend 40), ending the attack, and set both arm stiffnesses to 0.2.

#### Scenario: The recoil's stance starts at the blow
- **WHEN** a single-weapon striker's blow is turned
- **THEN** its attack ends on that tick in the combat idle with blend 40 and both arms at stiffness 0.2, and its executor stays busy until the recoil's end

#### Scenario: A blocked swing recoils unless two weapons hold it
- **WHEN** a released swing with the window bit and drive 1 is recoiled, drive 0.013 is recoiled, a class word of 0x90 with the continuation bit is recoiled, clock 0.4 with a counter is recoiled, clock 0.4 without a counter is recoiled, and the next lower clock with a counter is recoiled
- **THEN** drive 1 has strength and stall bits 0x3f000000, flags 0x27, end 1240, blend 40 and stiffness bits 0x3e4ccccd, drive 0.013 ends at 1200, the continuation and clock 0.4 with a counter leave no end, and clock 0.4 without a counter and the next lower clock end at 1240

## ADDED Requirements

### Requirement: Dual-wield counter
For a recoil or a follow-through of class word 0x90 without the continuation bit and with a clock widened past 0.4, the counter SHALL start when the flags (after the released bit clears) hold none of 0xA0012 and the attack is a side swing (motions 5 to 10): the other side's attack chains at once at full strength, with no stall and the continuation bit, and nothing else is written.

#### Scenario: Two weapons counter late in a side swing
- **WHEN** a striker of class word 0x90 is recoiled at clock 0.5 in motion 6, then in motion 11, then with the stalling bit
- **THEN** the first counters with no rest, and the other two recoil with a rest
