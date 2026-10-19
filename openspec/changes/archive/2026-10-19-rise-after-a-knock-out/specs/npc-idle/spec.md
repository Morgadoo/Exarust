# Spec Delta

## ADDED Requirements

### Requirement: Wake tone
While not waking, a tick SHALL change nothing. Through 1500 ms tone and target SHALL stay 0, the clip base SHALL be 0.4, and the condition SHALL be down with both flags set. At 1501 ms tone SHALL gain 0.001, or 0.002 getting up at phase 0, and the target SHALL stay 0. The dormant flag SHALL clear unless held. At 8000 ms the clip base SHALL stay. At 8001 ms it SHALL gain 0.01, and at 1 tone SHALL be 1, the target the stance factor, and flags SHALL clear unless held.

#### Scenario: Tone stays 0 for 1500 ms, then rises
- **WHEN** a knocked-out body is ticked at 1500 ms, at 1501 ms, and at 1502 ms, a getting-up body at phase 0 is ticked at 1501 ms, one at phase 1 is ticked at 1501 ms, a held body is ticked at 1501 ms, a body is ticked at 8000 ms, a body whose clip base is 0.995 is ticked at 8001 ms, a held body at that clip base is ticked at 8001 ms, and a body that is not waking is ticked at 9000 ms
- **THEN** 1500 ms stays at tone 0 with clip base 0.4 and both flags, 1501 ms is tone 0.001 with the dormant flag cleared and the target still 0, 1502 ms is tone 0.002, phase 0 is tone 0.002, phase 1 is tone 0.001, the held body keeps the dormant flag, 8000 ms does not move the clip base, 0.995 finishes at tone 1 with the stance factor as the target and the flags cleared, the held finish keeps the waking flag, and the body that is not waking is unchanged
