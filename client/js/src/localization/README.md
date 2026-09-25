# Browser localization

The BGS client reads the shared `preferences.locale` value. JSON catalogues in `client/js/src/localization` cover all 15 BGS languages. Keys are source UI strings; placeholders such as `{p0}` must be preserved. Run `node scripts/check-locales.mjs client/js/src/localization` and `node --test scripts/localization.test.mjs` after editing them. Initial translations were machine-assisted and should be reviewed for game terminology.

English keeps the original macroquad text renderer. Other languages use browser-shaped text cached as WebGL textures, so Devanagari shaping and Korean/Chinese glyphs work correctly. Text measurement and wrapping use the same font and translated text as drawing. Chat/state/actions are never translated or modified. Player names are excluded. Native desktop clients continue to use English.

`client/assets/localization` includes subsets of Noto Sans Devanagari and Noto Sans CJK, licensed under the SIL Open Font License; notices are included alongside the fonts. Regenerate the subsets after adding translation characters, using fontTools and the corresponding Noto source fonts. The Latin/Greek/Cyrillic renderer uses the existing Source Sans 3 asset.

For a BGS release, build the client against the engine version currently deployed. Keep the JS, WASM, fonts and other assets in the same immutable release directory. The browser smoke test should exercise the deployed engine's state format and at least one Latin, Devanagari, Korean and Traditional Chinese locale. Publishing this viewer must not change the engine package or game visibility.

With fontTools' WOFF support and the Noto source fonts installed, run `python scripts/build-localization-fonts.py` from the repository root. Font paths can be supplied with `--cjk-font` and `--devanagari-font`. Keep the OFL notices when redistributing regenerated fonts.
