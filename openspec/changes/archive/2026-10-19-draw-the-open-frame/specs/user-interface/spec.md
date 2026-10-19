## ADDED Requirements

### Requirement: Open frame
With the band bits and no wide bit, a clear hold bit SHALL force mask 15 and draw the wider strip. At unit 5 and height 100 the strip SHALL run from 10, 40 to 20, 60, the cap from 10, 20 to 20, 40, and the ledge from 20, 10 to 40, 20. A clear bit 1 SHALL count one call, and a set bit SHALL count none. The held mask SHALL stay 0. The helper body is not in.

#### Scenario: Height 100 sits at 10, 40
- **WHEN** unit 5 dispatches a form of height 100 that is not wide, with style bits 0x80 and the hold bit clear
- **THEN** the strip runs from 10, 40 to 20, 60 and the held mask is 0
