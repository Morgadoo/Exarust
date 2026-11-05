# Design

## Coin control

The control's draw message places the text baseline at the record's top
plus `(height + ascent + 2) >> 1` (the sum shifted as an unsigned 64-bit
word, low 32 bits kept). It draws, in the label group's sixth ink, each
present denomination's count (gold is the high 16 bits, then the silver and
copper bytes) four units after its place, the next place two units after
the text's end; then, in white at the GUI alpha, each present
denomination's icon at its place with its top the coin manager's cell
(`4 · unit`) above the baseline. The coin sheet's three 48-pixel cells
(at (0, 0), (48, 0), (96, 0)) are resampled into one atlas at that side.

`exa_items::item_gui::{coin_baseline, coin_cell, coin_draw}` hold the
rules; `exa_game::item_card::CoinSheet` the atlas and the draw.

## Preview bars

The loadout form's bind turns the character view's bars on and stores the
bound record's two health words over 65535 as the health bar's shares. The
view's draw callback paints the HUD's health and focus bars at the bottom
of the view (50 units left of its middle, 5 above its bottom; the focus bar
full) with the backgrounds at 220 alpha and no red floor. Exarust paints
them on the CPU through the HUD's painter and draws them as one quad over
the cleared preview. Exarust's roster member keeps one health word for both
pools (a stand-in).

## Label inks

The skin's label group starts at the palette offset that makes the skin
image's disabled ink its third shade and the socket ring's its eighth; the
rating dots' offset is then the seventh shade (index 6), which Exarust had
read as the eighth.

## Unreachable code

The unit registers three control classes (power slot 40 × 40, counter
28 × 16, bar 28 × 16). The only records naming them are one table (a
counter and five power slots) that no code references but the unit's
finaliser, and no code writes those class codes into a record at run time.
The power slot's release is the only caller of the technique selector's
opener. The HUD's centred message is drawn only under a flag that nothing
writes. These stay out; their modelled rules in `exa_ui::game_gui` are kept
and labelled.

## Open questions

- The loadout form's size in the original (answered by a retail capture of
  the hub at 1080p).
