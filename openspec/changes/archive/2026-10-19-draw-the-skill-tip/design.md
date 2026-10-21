# Design

## Behaviour

The method begins once, binds the skin's 0xe8 texture and makes two setup calls. It draws the row's icon handle at two units across and five units down. The label rectangle is x − 4u, y, of size 6u by 2u, using the row's text handle. The tooltip's x and y are fields 0x28 and 0x2c.

## Open questions

The skill tooltip table and its row fields are named only by position. The two setup calls' bodies are not in.

## Docs

This method is 222 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
