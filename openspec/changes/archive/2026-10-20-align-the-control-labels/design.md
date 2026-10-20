# Design

## Behaviour

Horizontal: `0x1000` places the label left of the control (`x − text_w − U`), `0x2000` centres it (`x + pad + U + trunc((avail−1−text_w)/2)`), `0x3000` right-aligns (`x+avail−text_w+U`), `0x4000` places it after the control (`x+avail+U`), default `x+pad+(U+1)/2`. Vertical: `0x100` lifts by `U/2`, `0x200` centres on the line, `0x300` sits on the base, `0x400` below the control, default half a unit.

## Model

`exa_ui::controls` adds `LabelAt` and `align_label`.

## Open questions

Text-run drawing and the control handlers.

## Docs

`RF_GUI_Controls_Ex` is **13%**. The headline rises slightly.
