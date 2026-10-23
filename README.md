# dirtygit

A terminal dashboard that finds the git repositories under the folders you choose and shows which ones have uncommitted or unpushed changes.

## Features

- Finds git repositories under the folders you list, down to a set depth
- Marks a repository as dirty when it has staged, unstaged, or untracked files, or commits not yet pushed to its upstream
- Lists the branch, ahead/behind counts, and last commit of each repository, and the changed files of the selected one
- Shows the results of the previous run at startup, then rescans in the background (every 5 minutes by default)
- Filters by name or by dirty state, and sorts dirty-first, by name, or by last commit. The sort and the dirty-only filter are kept between runs
- Opens [lazygit](https://github.com/jesseduffield/lazygit) in the selected repository
- Runs on Windows, macOS, and Linux

## Install

`git` must be on your `PATH`. `lazygit` is optional and only needed to open a repository with `Enter`.

### Scoop (Windows)

```
scoop bucket add yscoopy https://github.com/ycookiey/yscoopy
scoop install yscoopy/dirtygit
```

### Prebuilt binaries

Download the archive for your platform from [Releases](https://github.com/ycookiey/dirtygit/releases) (Windows x64, Linux x64, macOS x64 and Apple silicon), extract it, and put `dirtygit` on your `PATH`.

### Cargo

```
cargo install --git https://github.com/ycookiey/dirtygit
```

## Usage

Run `dirtygit` in a terminal. On the first run, a setup screen asks which folders to scan: type a path (`Tab` completes folder names), press `Enter` to add it, and press `Enter` on an empty line to save and start.

The setup screen writes `config.toml` to the settings folder, which you can edit later. The cache folder holds the last scan results and your sort and dirty-only filter choices.

| OS | Settings folder | Cache folder |
|----|-----------------|--------------|
| Linux | `~/.config/dirtygit` | `~/.cache/dirtygit` |
| macOS | `~/Library/Application Support/dirtygit` | `~/Library/Caches/dirtygit` |
| Windows | `%APPDATA%\dirtygit` | `%LOCALAPPDATA%\dirtygit` |

On Linux, `$XDG_CONFIG_HOME` and `$XDG_CACHE_HOME` replace `~/.config` and `~/.cache` when set.

```toml
scan_dirs = [
    "~/projects",
    "~/work",
]
exclude = ["node_modules", "vendor"]
interval_secs = 300
max_depth = 3
```

| Key | Description | Default |
|-----|-------------|---------|
| `scan_dirs` | Folders to search. A leading `~/` means your home folder | `["~/projects"]` |
| `exclude` | Folder names to skip | `[]` |
| `interval_secs` | Seconds between background rescans | `300` |
| `max_depth` | How many folder levels below each `scan_dirs` entry to search | `3` |

On Windows, write paths with forward slashes (`"C:/src"`) or in single quotes (`'C:\src'`), since TOML treats `\` in double quotes as an escape.

### Keys

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `G` | Jump to top / bottom |
| `Ctrl+d` / `Ctrl+u` | Half page down / up |
| `Ctrl+f` / `Ctrl+b` | Full page down / up |
| `Tab` | Switch between the list and the detail pane |
| `/` | Search by repository name (`Enter` keeps the filter, `Esc` clears it) |
| `s` | Cycle sort mode |
| `f` | Toggle the dirty-only filter |
| `c` | Copy the repository path to the clipboard |
| `r` | Rescan now |
| `Enter` | Open lazygit in the selected repository |
| `?` | Show or hide help |
| `q` | Quit |

The mouse wheel scrolls the active pane.

## Known limitations

- Folders whose names start with `.` are not searched, and a repository inside another repository is not listed
- If `lazygit` is not on your `PATH`, `Enter` opens nothing
- On Linux, `c` needs `xclip` or `xsel`

## Uninstall

Remove the binary (`scoop uninstall dirtygit` if you used Scoop), then delete the settings and cache folders listed under [Usage](#usage).

## Build

```
cargo build --release
```

The binary is written to `target/release/`.

## License

[MIT](LICENSE)
