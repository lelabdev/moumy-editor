# Moumy Editor

Recipe editor for [Moumy's handwritten recipes](https://github.com/lelabdev/moumy-recettes). A standalone Rust binary that reads and writes `.md` recipe files directly — no database, no server runtime.

## Quick Start

```bash
# Build
cargo build --release

# Run from the recipes directory
cd /path/to/moumy-recettes/src/data/recettes
moumy-editor
```

Or set the directory explicitly:

```bash
RECIPES_DIR=~/dev/moumy-recettes/src/data/recettes moumy-editor
```

Opens at `http://localhost:3210`.

## Features

- **Recipe explorer** — browse all recipes with search
- **Form editor** — edit frontmatter (title, category, difficulty, times, servings, ingredients, notes)
- **Step editor** — add/remove/reorder preparation steps
- **Direct .md read/write** — no database, files are the source of truth
- **Cross-platform** — compiles on Linux, macOS, Windows

## Recipe Format

Recipes are standard Markdown files with YAML frontmatter:

```yaml
---
title: 'Gâteau à l\'orange'
category: desserts
categoryLabel: Desserts
prepTime: 15
cookTime: 35
servings: 8
difficulty: Facile
ingredients:
  - 250g de farine
  - 125g de beurre mou
notes: "Remplacer les oranges par des citrons."
---

1. Mélanger en crème le beurre et le sucre.
2. Ajouter les œufs entiers.
3. Cuire à four modéré.
```

## Tech

- **Rust** + **axum** — HTTP server
- **serde_yaml** — YAML frontmatter parsing
- **Tailwind CSS** (CDN) — UI styling
- **Vanilla JS** — no build step, single HTML file embedded in binary

## License

MIT
