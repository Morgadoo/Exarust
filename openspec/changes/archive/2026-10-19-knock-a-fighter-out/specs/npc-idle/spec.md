# Spec Delta

## ADDED Requirements

### Requirement: A knock-out is the unconscious state
A knock-out SHALL set the wake clock to the current clock plus the delay, wrapping. Flags SHALL lose bits 0xF0 and gain the dormant and waking flags. The condition SHALL be down, the statue mark SHALL clear, gait SHALL be 0, and tone SHALL be 0. The pin gain bits SHALL be 0x3da3d70a. A sleep countdown below 256 SHALL rise to 256, and one already at 300 SHALL stay. A blow SHALL NOT call this, and health SHALL NOT store a limb impairment.

#### Scenario: Delay 40 at clock 1000
- **WHEN** a statue with gait 2, tone 1, flags 0x80000150 and a sleep countdown of 300 is knocked out at clock 1000 with delay 40, and the same body with a lower countdown is knocked out at the maximum clock with delay 1
- **THEN** the first wakes at 1040, is down, has flags 0x80000130, is not a statue, has gait 0 and tone 0, has pin bits 0x3da3d70a, and keeps countdown 300, and the second wakes at the minimum clock with countdown 256
