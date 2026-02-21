![Plum Chess Logo](PlumChessLogo.png)

# Plum Chess Engine — How to Play

Plum Chess is a chess engine that implements the standard **UCI (Universal Chess Interface)** protocol, the same interface used by popular engines like Stockfish. To play against Plum Chess, you’ll need a compatible chess GUI (graphical front-end).

---

## Obtaining the Plum Chess Executable

You have two options: download a prebuilt binary or build the engine from source.

### Option 1: Download a Prebuilt Binary

Precompiled executables are published on GitHub under **Releases** (tagged versions).

1. Open the project **Releases** page.
2. Click the latest tagged release (`vX.Y.Z`).
3. In **Assets**, download the binary matching your OS/architecture.

Asset naming format:

```
plum_chess-v<version>-<os>-<arch>[.exe]
```

Examples:

* `plum_chess-v3.0.0-windows-x86_64.exe`
* `plum_chess-v3.0.0-linux-x86_64`
* `plum_chess-v3.0.0-macos-aarch64`

You can also browse tags directly from the **Tags** tab, then open the corresponding release.

### Option 2: Build from Source

If you prefer to build Plum Chess yourself, you’ll need an existing Rust installation.

From the root of the repository, run:

```bash
cargo build --release
```

Once the build completes, the executable will be located at:

```
target/release/plum_chess
```

(On Windows: `target/release/plum_chess.exe`.)

You may wish to copy this file to a more permanent or convenient location on your system.

---

## Playing with PyChess

[PyChess](https://pychess.github.io) is a popular and user-friendly GUI that supports custom UCI engines.

### Installing PyChess

Download and install PyChess from the official website:

* [https://pychess.github.io/download/](https://pychess.github.io/download/)

Launch PyChess and, if prompted, click through the initial splash screen dialogs.

### Adding Plum Chess as an Engine

1. In PyChess, navigate to:

   ```
   Edit → Engines → New
   ```
2. Browse to and select the `plum_chess` executable.
3. PyChess will automatically detect that the engine uses the **UCI** protocol.
4. Fill in any engine metadata as desired.
5. Click **Save**.

### Starting a Game

1. Return to the main PyChess screen.
2. In the top-left selection panel:

   * Choose your color.
   * Select **Plum Chess** as your opponent.
   * Set the desired difficulty level.

You may also click the **weather icon** to configure additional game parameters such as time limits.
⚠️ **Note:** Plum Chess 2.0 is not optimized for timed play and will generally waste available time rather than managing it efficiently.

---

## Difficulty Levels

Plum Chess now uses a mixed ladder:

- `1`: Random engine (diagnostic baseline)
- `2`: Greedy engine (capture-first style)
- `3..=17`: Humanized CPL engine (`engine_humanized_v5`)
- `18`: Iterative v16 (best-move focused), depth `8`
- `19`: Iterative v16 (best-move focused), depth `12`
- `20+`: Iterative v16 (best-move focused), depth `16`

For humanized levels (`3..=17`), the engine:

- Runs normal search and collects top root candidates (MultiPV-style root set)
- Computes centipawn loss (CPL) relative to the best move
- Uses a strength percentage that scales linearly from:
  - level `3` -> `60%`
  - level `17` -> `100%`
- Chooses moves with a CPL budget model, then applies linear weighted randomness
  so lower levels play more human-like inaccuracies while higher levels converge
  to stronger choices.

## Documentation Directory

The `docs/` directory contains the project guides and roadmap history:

- `docs/code_structure.md`
  - Current architecture and module interaction map.
- `docs/optimization.md`
  - Search and engine optimization evolution guide.
- `docs/uci_enhancement.md`
  - UCI implementation and compliance journey.
- `docs/multithread_roadmap.md`
  - Major version 4 multi-threading roadmap and outcomes.
- `docs/humanizing.md`
  - Major version 5 humanized CPL strategy guide.
- `docs/end_game_optimization.md`
  - Major version 6 endgame optimization process and acceptance outcomes.
- `docs/requirements/v5.md`
  - Locked requirements/spec for the v5 humanized engine effort.
- `docs/requirements/v6.md`
  - Locked requirements/spec for the v6 endgame-strength effort.
- `docs/engine-interface.txt`
  - Reference UCI protocol specification text.

## Developer Notes

For thread scaling measurements, run:

```bash
cargo run --bin thread_scaling_bench -- 8 4 3
```


## Enjoy!

That’s it—set it up, experiment with the levels, and have fun playing against Plum Chess ♟️
