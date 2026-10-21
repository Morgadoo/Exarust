# Design

## Behaviour

Both sentence builders load the byte, convert it to a double, divide by the
stored 2.55 and convert back with the rounding conversion, which follows the
processor's default rounding: nearest, ties to even.

## Model

`scale_mark` divides by the same stored double and rounds with
`round_ties_even`.

## Open questions

None.

## Docs

None describe the marks beyond the spec.
