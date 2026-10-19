# Design

## Behaviour

After the colour word, the writer masks the flag with 4 and keeps the mark only when that result is greater than 0. The masked result is 0 or 4, so the high bit of the flag does not hide bit 4. Each of the twelve corners stores the mark word at the current cursor, then the cursor advances by one with wrap. A clear bit stores nothing and still advances.

The image does not store the flag or the mark, so a fresh read is 0 and no mark is written. Those words, and the cursor, are arguments because the writer reads them live.

## Open questions

None in this writer. The other methods of the line class are not in.

## Docs

The writer is shared, so these bytes are not added to the line class. `TGSLines` stays 1%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
