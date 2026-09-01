<div align="center">
  
# 📖 Vorto

**A lightning-fast, modal terminal Bible reader written in Rust.**  
*Inspired by the fluidity of Vim, `oil.nvim`, and Telescope.*

[![Built with Nix](https://img.shields.io/badge/Built_with-Nix-5277C3?style=flat-square&logo=nixos&logoColor=white)](#installation)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](LICENSE)

</div>

---

## ⚡ Features

Vorto brings modern Neovim-style navigation and fuzzy finding to your daily reading:

- **Telescope-Style Quick Jump**: Press `<Space>` from anywhere to open a fuzzy-finding jump menu. Type `gen 3 15` and hit enter to instantly teleport.
- **Hierarchical Drill-Down**: Navigate Books -> Chapters -> Verses just like a file explorer.
- **Fuzzy Filtering**: Press `/` in any view to instantly filter the list. Press `<Enter>` to select an item and lock onto it in context.
- **Jump History**: Made a mistake or want to refer back? Press `<C-o>` and `<C-i>` to effortlessly jump backward and forward through your navigation history.
- **Visual Yanking**: Press `v` to enter visual mode, use `j`/`k` to select multiple verses, and `y` to instantly copy them to your system clipboard (flawless Wayland and X11 support out of the box).
- **Translation Manager**: Pre-packaged with an embedded, lightning-fast SQLite FTS5 database containing 7 translations. Press `T` to open a GUI to hide or re-order your favorite translations.

---

## 📚 Included Translations

Vorto builds its own localized database entirely offline using data provided by [eBible.org](https://ebible.org).

* **BSB** (Berean Study Bible) - *English*
* **KJV** (King James Version 2006) - *English*
* **WEB** (World English Bible) - *English*
* **LSV** (Literal Standard Version) - *English*
* **NOBLB** (Biblica® Open En Levende Bok) - *Norwegian Bokmål*
* **VUC** (Latin Vulgate Clementine) - *Latin*
* **EPO** (Esperanto Londono) - *Esperanto*

> Want to add more? Simply download the `.usfm.zip` from eBible.org into the `data/` directory and add it to the `flake.nix` build script!

---

## 🚀 Installation

Vorto is packaged using Nix flakes. 

To build and run the app directly from the source repository without installing:
```bash
nix run .
```

To build a persistent binary in your nix store:
```bash
nix build
./result/bin/vorto
```

---

## ⌨️ Keybindings

Vorto is designed for people who prefer their hands on the home row.

| Key(s) | Action | Context |
| :--- | :--- | :--- |
| `j` / `k` / `Down` / `Up` | Move cursor up and down | Global |
| `<C-d>` / `<C-u>` | Page down / Page up | Global |
| `g` `g` / `G` | Jump to top / bottom of list | Global |
| `<Enter>` | Open selected Book/Chapter, or jump to Search Result | Global |
| `-` | Go up one level (e.g. Verses -> Chapters) | Global |
| `[` / `]` | Jump to previous / next Chapter | Verses & Chapters |
| `{` / `}` | Jump to previous / next Book | Verses & Chapters |
| `<C-o>` / `<C-i>` | Jump backward / forward through history | Global |
| `<Space>` | Open Telescope-style Quick Jump menu | Global |
| `/` | Start fuzzy filtering the current list | Global |
| `S` or `?` | Start a Global Search across the whole Bible | Global |
| `t` or `c` | Open quick translation switcher | Global |
| `T` | Open Translation Manager (Reorder/Hide translations) | Global |
| `v` | Start visual selection mode | Verses |
| `y` | Yank (copy) selected verses to system clipboard | Verses (Visual Mode) |
| `<Esc>` / `q` | Clear filters, cancel visual mode, or quit app | Global |

### 🛠️ Menu Controls

**Filter Mode (`/`)**
- `<C-n>` / `<C-p>`: Navigate filtered items
- `<Enter>`: Drill into the selected item and clear the filter
- `<Esc>`: Cancel and clear filter

**Translation Manager (`T`)**
- `J` / `K` (Shift): Drag a translation up or down to change its order
- `<Space>`: Toggle visibility (hide/show)
- `<Enter>` / `<Esc>`: Save preferences and close

---

## ⚖️ License

The **source code** for this application is licensed under the [MIT License](LICENSE).

The **Bible data** embedded within this application is derived from [eBible.org](https://ebible.org). Most included translations (BSB, KJV, WEB, LSV, VUC, EPO) are in the Public Domain. The Norwegian Living NT (`NOBLB`) is provided under the [Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/). See the `LICENSE` file for full details.
