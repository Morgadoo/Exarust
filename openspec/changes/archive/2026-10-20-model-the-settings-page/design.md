# Design

## Behaviour

The Settings tab has fifteen items: two info rows, one slider, seven choices and five toggles. The toggles share rows 10–12, with two columns on rows 10 and 11 and the last one alone. The choices index `["Off","Low","Medium","High","Very high"]`. The Controls tab has fourteen label/key rows.

The reference form is 630×800 at (500,130) in a 1600×1080 client. The tabs are (20,46,280,34) and (308,46,280,34); the content starts at (4,92); the settings rows start at offsets 130, 171, 229, 285, 323, 361, 399, 437, 475, 529, 574, 616 and 658; the controls rows at 130, 164, 198, 231, 265, 298, 332, 366, 399, 433, 466, 500, 534 and 568. Columns are at 40/235/460 for settings and 130/420 for controls. The footer buttons are (60,723,140,28), (225,723,145,28) and (390,723,150,28). Other form sizes scale linearly.

## Model

`exa_ui::settings` exposes the page content and `layout(form)`.

## Open questions

The embedded control-template records, the controls-list scrolling and the apply/reset logic.

## Docs

No coverage change; the page is screenshot-derived.
