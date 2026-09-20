# Fast Diary

Diary app built with Druid library and Rust language.

```bash
# Linux Bash
RUST_LOG='debug' cargo watch -x run
```

```powershell
# Windows powershell
$env:RUST_LOG='debug'; cargo watch -x run; $env:RUST_LOG=$null
```

## Features

### Folders and diaries

- A diary is a file in the folder you choose, named after its date and time: `yymmddHHMMSS.md`
  (`.md.enc` in an encrypted folder).
- The first screen has the folder chooser and the history of chosen folders on the left, and the
  password input with the button that opens the folder on the right. The content has a fixed width
  of 800 px and stays centered in a wider window. Long folder paths are shortened to 40 characters
  by replacing their middle with five dots; the full path is still what gets selected.
- The opened diary is saved with <kbd>Ctrl</kbd>+<kbd>S</kbd>, every 5 seconds while it changes,
  when another diary is opened, when the folder is closed and when the window is closed.
- The **+** button (and <kbd>Ctrl</kbd>+<kbd>N</kbd>) creates a diary for the current moment. Only one
  diary can be created per hour: when the current hour already has one, that diary is opened
  instead. The diary that was open is saved first.

### Calendar and list

- The button at the right end of the toolbar (icon and name) switches between the diary **list** and
  the **calendar**. The choice is remembered.
- The calendar shows three months one below the other. The arrows move it by three months, and the
  **Today** button selects today. When a diary of a month that is not shown is opened, the calendar
  moves so that this month is in the middle.
- Days that have diaries are marked, with up to three dots for the number of diaries. Clicking a day
  opens its latest diary, and clicking it again opens the next older one, then starts over.
- Clicking a day that has no diary opens an empty **draft** for that day at `00:00:00`. The file is
  created only when something is written and saved.
- With the calendar focused, the arrow keys move to the previous and the next diary.

### Editor and markdown preview

- The button on the title row switches between editing, editing next to the preview, and the
  preview only (an empty, a half filled and a filled square). The choice is remembered.
- The title shows the date and time in words in the chosen language, for example
  `15 October 2026 12:13:14` or `15 Ekim 2026 12:13:14`.
- The preview supports headings, bold, italic, strikethrough, inline code, code blocks, block quotes,
  bullet and numbered lists (also nested), task lists, horizontal rules and links. Links are shown
  but do not open.
- The preview is updated 0.6 seconds after the text was last changed, and at once when another diary
  is opened, because laying out a long text on every key press is slow.

### Window and layout

- The window is at least 800x600 and starts with that size the first time.
- The position, size and maximized state of the window and the positions of the two draggable bars
  (diary list and editor, editor and preview) are remembered. A window that was closed maximized
  opens maximized and goes back to its last normal size, or to 1000x700 when that is unknown. A
  window whose remembered position is outside of every monitor is moved back into view.
- The diary list is at least 200 px wide.

### Menu, toolbar and About

- Menu bar: **File** (new diary, open folder, save, close folder, settings, quit), **Edit** (cut,
  copy, paste, select all), **View** (list or calendar, editor mode, language) and **Help**.
  Shortcuts: <kbd>Ctrl</kbd>+<kbd>N</kbd>, <kbd>Ctrl</kbd>+<kbd>O</kbd> (on the first screen),
  <kbd>Ctrl</kbd>+<kbd>S</kbd> and <kbd>Ctrl</kbd>+<kbd>Q</kbd> (<kbd>Cmd</kbd> on macOS).
- The toolbar above the diaries has icon buttons for creating a diary, the settings and closing the
  folder. A tooltip names each of them.
- **Help > About** shows the version, the author, the repository, the address for reporting issues
  and the addresses for supporting the project, each with a button that copies it.

### Languages

- English and Türkçe. **View > Language** offers these two and *System Default*, which follows the
  language of the operating system. The texts change at once and the choice is remembered.
- The texts, including the names of the months and of the weekdays, are in
  `resources/i18n/<locale>/builtin.ftl`. They are built into the program, and a text that a
  language lacks is taken from English.

## Download and releases

Every push to `main` and every pull request runs the checks (format, clippy and tests) in
`.github/workflows/build.yml`. Every push to `main` also builds the program for these systems, and
the builds can be downloaded from the run under the *Actions* tab of the repository (they are kept
for 90 days):

| Build                 | System                                |
|-----------------------|---------------------------------------|
| `linux-x86_64`        | Linux, needs GTK 3 on the computer    |
| `windows-x86_64`      | Windows                               |
| `macos-apple-silicon` | macOS on Apple Silicon (M1 and newer) |
| `macos-intel`         | macOS on Intel processors             |

To publish a version, raise `version` in `Cargo.toml`, commit it, and push a tag with the same
number (the build stops when they differ):

```bash
git tag v0.1.0
git push origin v0.1.0
```

The builds of that tag are then published on the *Releases* page of the repository, together with
generated release notes. Each archive has the program, this README and the license. The images and the texts are
built into the program, so it can be started from any folder.

The programs are not signed. Windows shows a *SmartScreen* warning (choose *More info* and *Run
anyway*), and macOS blocks the first start (open it with a right click and *Open*, or run
`xattr -d com.apple.quarantine fast-diary`).

## Settings file

Everything that is remembered is in one JSON file:

| System  | Path                                                  |
|---------|-------------------------------------------------------|
| Linux   | `~/.config/fast-diary/settings.json`                  |
| macOS   | `~/Library/Application Support/fast-diary/settings.json` |
| Windows | `%APPDATA%\fast-diary\settings.json`                  |

```json
{
  "recent_folders": ["/home/me/diary"],
  "diary_view_mode": "Calendar",
  "editor_mode": "Split",
  "language": "System",
  "window": {
    "normal": { "x": 100.0, "y": 100.0, "width": 1000.0, "height": 700.0 },
    "maximized": false
  },
  "layout": { "list_split_percent": 30.0, "editor_split_percent": 50.0 }
}
```

A missing or broken file is replaced by the defaults, and every field is optional.

## Project layout

| Path                | Content                                                              |
|---------------------|----------------------------------------------------------------------|
| `src/config`        | Settings file, window and layout settings                            |
| `src/modal`         | Application state: diaries, calendar months, modes, language         |
| `src/storage`       | Reading, writing and history of diary files                          |
| `src/vault`         | Encryption of folders                                                |
| `src/view`          | Pages, windows, menu and widgets (calendar, markdown preview, split) |
| `src/utils`         | Translations, text helpers and other small helpers                   |
| `resources/i18n`    | Translations                                                         |
| `tests`             | Integration tests                                                    |

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The integration tests drive the real widgets without a window, using the test harness of Druid: they
click days, press keys, drag the bars and fire timers. The translations of the menu, the calendar and
the About window are checked for gaps in every language by `tests/localization_resources.rs`.

## Known limitations

- Druid lays out the whole text of a diary again on every key press. This takes about 18 ms for a
  50 KB diary and about 110 ms for a 200 KB diary, so very long diaries feel slow to edit.
- Some texts are still English only: the form for encrypting a folder, the settings page and the
  title of the dialog that asks for a folder.
- Wayland does not let an application place its window, so the remembered position has no effect
  there. The size and the bars are restored.

## License

Copyright (C) 2026 Emir Buğra Köksalan

Fast Diary is free software: you can redistribute it and change it under the terms of the GNU
General Public License, version 3 or (at your option) any later version. It is distributed in the
hope that it will be useful, but without any warranty. The full text is in the file
[LICENSE](LICENSE).

## Contact

Emir Buğra Köksalan, <kodmanyagha@gmail.com>

- Repository: <https://github.com/kodmanyagha/fast-diary>
- Report an issue: <https://github.com/kodmanyagha/fast-diary/issues>

## Tasks

- [x] Selecting unencrypted folder.
- [x] Saving diaries to file.
- [ ] Selecting encrypted folder.
- [ ] Open password screen when window blur.
- [x] Markdown rendering (edit, split and preview modes).
- [x] Calendar view next to the diary list.
- [x] Remembering window position, size, maximized state and split positions.
- [x] Menu bar (File, Edit, View, Help).
- [x] English and Türkçe, chosen from the View menu.
- [x] About window with the contact and the support addresses.
