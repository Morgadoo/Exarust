# Spec Delta

## ADDED Requirements

### Requirement: Run-time body links
A link added to a body after it is built SHALL weight each point over its part's rest particles, take the two link-mobility shares, the pivot softness 0.001 and the body and code flags, store a zero slack when the slack in centimetres is at or under the general epsilon or is a NaN, start with a zero smoothed length and zero smoothing, and go at the end of its part's links. A link to a scene node SHALL take the code flag alone.

#### Scenario: Tiny slack
- **WHEN** a link is added with a slack of 1e-9 m or a NaN slack
- **THEN** its slack is zero, and a slack of 1.2e-9 m is kept

### Requirement: Two-step point transform
The two-step point transform SHALL be the point's place over the current particles less its place over the third particle set, each summed as (w0p0 + w1p1) + (w2p2 + w3p3). The third set SHALL take the tick's start in the post-step and the current particles on the first tick of sleep.

#### Scenario: Paired sums
- **WHEN** the particles' x are 1e8, 1, -1e8 and 1, the third set is zero and every weight is 1
- **THEN** the displacement's x is 0

### Requirement: Disabled body
A disabled body SHALL meet no other body in the pair stage, gather no level contacts, and break a grab on it.

#### Scenario: Disabled pair
- **WHEN** one of two overlapping balls is disabled
- **THEN** the pair stage finds no touch

### Requirement: Tick take
Taking ticks off a body's countdown SHALL subtract them and hold a result under two at one.

#### Scenario: Large take
- **WHEN** 300 ticks are taken from a countdown of 253
- **THEN** the countdown is 1
