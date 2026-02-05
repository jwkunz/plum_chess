# Plum Chess Engine — How to Play

Plum Chess is a chess engine that implements the standard **UCI (Universal Chess Interface)** protocol, the same interface used by popular engines like Stockfish. To play against Plum Chess, you’ll need a compatible chess GUI (graphical front-end).

---

## Obtaining the Plum Chess Executable

You have two options: download a prebuilt binary or build the engine from source.

### Option 1: Download a Prebuilt Binary

Precompiled executables are available in the **Releases** section of this repository.
Currently supported platforms:

* **Windows (Win32)**
* **Linux (x86_64)**

Simply download the appropriate binary for your system.

### Option 2: Build from Source

If you prefer to build Plum Chess yourself, you’ll need an existing Rust installation.

From the root of the repository, run:

```bash
cargo build --release
```

Once the build completes, the executable will be located at:

```
target/release/plum_chess_X.X.X_YYY.exe
```

You may wish to copy this file to a more permanent or convenient location on your system.

---

## Playing with PyChess

[PyChess](https://pychess.github.io/download/) is a popular and user-friendly GUI that supports custom UCI engines.

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

Plum Chess 2.0 supports difficulty levels **0 through 7**, each corresponding to a different playing strategy:

* **Level 0** — Fully random move generation
* **Level 1** — Greedy play (always attacks)
* **Levels 2–7** — Iterative deepening using a conventional material-based evaluation, searching progressively deeper layers

The highest level plays at approximately level 1800 ELO as determined by centipawn loss using the the Chessis analysis toolkit over candidate 10 games against stockfish.

---

## Enjoy!

That’s it—set it up, experiment with the levels, and have fun playing against Plum Chess ♟️


