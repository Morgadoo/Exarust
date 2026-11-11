# runtime-library Specification

## Purpose
The Free Pascal run-time library behaviour the shipped game's own code reaches and whose results Exarust reproduces: the random generator behind `RandSeed` and its three `Random` forms.

## Requirements

### Requirement: Run-time library unit draw
The run-time library's `Random` SHALL be the next MT19937 output times 2⁻³² in double precision, with MT19937's standard seeding, twist and tempering.

#### Scenario: Seed 5489
- **WHEN** a stream seeded with 5489 is drawn once
- **THEN** the output word is 3499211612 and the unit draw is 3499211612 · 2⁻³²

### Requirement: Run-time library whole-number draw
`Random(l)` for a 32-bit `l` SHALL raise a negative `l` by one, multiply the draw by `l` as a signed 64-bit product and return the product's high word.

#### Scenario: A negative bound
- **WHEN** the draw is 3499211612 and `l` is −10
- **THEN** the result is −8

### Requirement: Run-time library 64-bit draw
`Random(l)` for a 64-bit `l` SHALL take the first draw as the low word and the second, its top bit cleared, as the high word, and return that value's signed remainder by `l`, or 0 when `l` is 0.

#### Scenario: Remainder by 1000
- **WHEN** the draws are 3499211612 then 581869302 and `l` is 1000
- **THEN** the result is ((581869302 · 2³²) + 3499211612) mod 1000

### Requirement: RandSeed reseed rule
The process-wide generator SHALL start with `RandSeed` 0 and its word index past the end. A draw SHALL reseed from `RandSeed` when `RandSeed` differs from the value the last reseed left or the index is past the end, and a reseed SHALL store the seed's complement in `RandSeed`.

#### Scenario: A fixed seed stored twice
- **WHEN** `RandSeed` is set to 5489, a draw is made, and it is set to 5489 again
- **THEN** both draws are 3499211612

#### Scenario: The complement stored back
- **WHEN** after a reseed from 5489 `RandSeed` is set to its complement
- **THEN** the next draw continues the stream

### Requirement: Randomize
`Randomize` SHALL store the tick count in `RandSeed`, so the next draw reseeds from it.

#### Scenario: Tick count 123456
- **WHEN** `Randomize` runs at tick count 123456
- **THEN** the next draw equals a stream seeded with 123456's first output
