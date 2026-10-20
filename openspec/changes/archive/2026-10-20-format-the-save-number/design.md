# Design

## Behaviour

A zero value takes the literal `"000"`. A negative value never enters the digit loop, so it also ends at `"000"`.

Positive values collect their decimal digits from the least significant up. After the loop, a length under two prefixes `"00"` and a length under three prefixes `"0"`, giving the zero-padded three-character form. A longer value is untouched.

## Model

`format_save_number(value)` returns the padded string. The original builds it through the string helper; the model builds the same text.

## Open questions

The string helper is not in.

## Docs

This method is 258 bytes. `XA_SaveGames` moves from 6% to about 7%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
