# Spec Delta

## ADDED Requirements

### Requirement: Span arrival
A walker not approaching a span SHALL stay as it is. While the clock is still at the stamp and the walker is busy, the approach SHALL stay as it is. Otherwise a missing span, or a horizontal distance past 250 cm from the midpoint, SHALL send the walker back to seeking. The first miss SHALL set only the high skipped bit. A later miss SHALL also skip the cursor's low five bits. A distance of 250 cm SHALL start the door wait at the current clock. The aim snap after the choice is not in this rule.

#### Scenario: Two hundred fifty centimetres starts the wait
- **WHEN** a walker already at a door is stepped far past a span, a busy approach at its stamp is stepped past the span, an approach at its stamp that is not busy is stepped at exactly 250 cm, a busy approach one millisecond later meets no span at cursor 3, that approach meets a point 251 cm out again, and an approach meets the midpoint one millisecond later while busy
- **THEN** the walker at the door stays there, the busy approach at its stamp stays there with no skip, the point at 250 cm starts the door wait at that clock, the missing span seeks and sets only the high bit, the later miss also skips bit 3, and the midpoint starts the door wait at the later clock
