# M22 production-cell raster evidence

These six PNGs are deterministic review renderings of Yoctui's exact
`160x50` Ratatui cell/style goldens. They are generated evidence, not a source
of UI state. M25 promotes their reviewed shell geometry, scene composition,
color roles, editor layout, and anchored menu placement into the executable's
visual acceptance target; M21 remains the visual direction.

The renderer pins PyCairo/Cairo, DejaVu Sans Mono regular and bold font hashes,
cell geometry, antialiasing, hinting, every source-cell SHA-256, and every PNG
SHA-256 in [`manifest.toml`](manifest.toml).

Reproduce and verify them with:

```bash
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py
```

After reviewing an intentional production cell/style change, regenerate with
`./scripts/render-m22-concept-screenshots.sh --update` and inspect every PNG,
source hash, and output hash diff.
