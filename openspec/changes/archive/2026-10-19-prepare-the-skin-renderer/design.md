# Design

## Behaviour

The method checks a global flag. When it is not 1, it calls the mode setup with 5 and then the second setup, and returns 1. When it is already 1, it skips both calls and returns 1.

## Open questions

The two setup bodies are not in.

## Docs

This method is 51 bytes. `TGUIDrawSkin` stays 32%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
