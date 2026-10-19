# Design

## Behaviour

`TGUIDrawSkin`'s slot 27 is 0x1002198c0. `TGSLines`'s slots list the same address at index 27 and does not list it as an own override, so the method belongs to the parent. The method selects its texture set from two booleans: the first true takes the raised set, otherwise the second true takes the pressed set, otherwise the idle set. `draw_bevel` already implements that selection from state bits 0x40 and 0x10, and its fills, border and outline are tested.

`draw_skin_bevel` accepts the two booleans directly, maps the first to bit 0x40 and the second to bit 0x10, and delegates. This keeps the parent's signature visible without duplicating the geometry.

## Open questions

The two callers that produce the booleans are not in.

## Docs

This method is 775 bytes. `TGUIDrawSkin` is 42%. `TGSLines` stays 87% of its own overrides. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
