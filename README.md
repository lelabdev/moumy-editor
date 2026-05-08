# Moumy Editor ✍️

A recipe editor for preserving handwritten family recipes. Built as a tribute to my grandmother — her notebooks full of love, butter, and carefully measured ingredients deserved more than a dusty drawer.

The editor powers [**moumy.ovh**](https://moumy.ovh), a static site that publishes her recipes for the family to keep.

> *"Un hommage à sa vie, à sa cuisine, à ses petits souvenirs écrits à la main."*

## What it does

Moumy Editor is a standalone desktop tool that reads and writes recipe `.md` files directly — no database, no server, no cloud. Just markdown files in a folder.

- 📝 **Recipe editor** — title, category, ingredients, steps, notes, difficulty, servings
- 📷 **Manuscript viewer** — see the handwritten original side-by-side with the transcription
- 🔍 **OCR transcription** — Mistral OCR to transcribe handwritten recipes into text
- 📤 **Git integration** — push changes to GitHub from the editor
- 🆕 **Auto-updater** — checks for new versions on startup
- 🖥️ **Cross-platform** — Linux & Windows binaries

## Quick Start

### Download

Grab the latest binary from [**Releases**](https://github.com/lelabdev/moumy-editor/releases):

| Platform | File |
|----------|------|
| Linux x86_64 | `moumy-editor` |
| Windows x86_64 | `moumy-editor.exe` |

### Run

```bash
# From the recipes directory
cd /path/to/moumy-recettes
./moumy-editor
```

Or set the directory explicitly:

```bash
RECIPES_DIR=/path/to/recipes ./moumy-editor
```

Opens at **http://localhost:3210**.

### Optional: OCR

For handwritten recipe transcription, create a `.env` file next to the binary:

```
MISTRAL_API_KEY=your-key-here
```

## Recipe Format

Recipes are Markdown files with YAML frontmatter:

```yaml
---
title: "Gâteau à l'orange"
category: desserts
prepTime: 15
cookTime: 35
servings: 8
difficulty: Facile
ingredients:
  - 250g de farine
  - 125g de beurre mou
sourceImage: Gâteau_A
---

1. Mélanger en crème le beurre et le sucre.
2. Ajouter les œufs entiers en battant la pâte.
3. Verser dans un moule beurré et cuire à four modéré.
```

## Tech Stack

| Layer | Choice |
|-------|--------|
| Backend | **Rust** + **axum** |
| Frontend | Vanilla JS, single HTML file embedded in binary |
| Styling | Tailwind CSS (CDN) |
| Data | YAML frontmatter in `.md` files |
| OCR | Mistral OCR API |
| Site | [SvelteKit](https://github.com/lelabdev/moumy-recettes) → static HTML on Cloudflare |

## Why

My grandmother spent decades writing recipes in old notebooks. Faded ink, stained pages, her handwriting getting softer over the years. This project is about keeping those recipes alive — not just the ingredients and steps, but the fact that she wrote them down for us.

The site is at [**moumy.ovh**](https://moumy.ovh). The editor is how we get her recipes from paper to screen.

## License

MIT
