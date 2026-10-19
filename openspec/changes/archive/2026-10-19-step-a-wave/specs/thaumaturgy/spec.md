# Spec Delta

## ADDED Requirements

### Requirement: Wave front
A wave's rate SHALL be the tick in seconds, then five thirds of that. While charging or fizzling, the radius SHALL grow by 0.0015. Once released, the reach SHALL grow by 0.6, the radius by 0.0006 of the speed, and the trail step SHALL be 0.07 of the speed plus 1. An amount of 0.99 with a 12 ms wave rate SHALL release. The blast rate for that same tick SHALL leave the amount at 1.

#### Scenario: A 12 ms wave and a still front
- **WHEN** 0.99 is charged by a 12 ms wave rate and by the blast rate, a radius grows while charging, and a released front of speed 0 and of speed 10 is stepped
- **THEN** the wave releases, the blast amount stays at 1, the charging radius is 0.0015, the still front has reach 0.6 and trail 1, and speed 10 grows the radius by 0.006 with trail 1.7
