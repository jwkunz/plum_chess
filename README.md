# How to play
Plum chess is an engine that follows the same `uci` interface that the popular Stockfish chess engine follow.  To play this engine, you need a front end graphical environment.


# Obtaining the Plum Chess Executable
One can simply download the pre-build executable from the releases page.  I have uploaded precompiled binaries for for Win32 and Linux x86_64 architectures.

Alternatively, one can build this rust project from source by using and existing rust installation with `cargo build --release` from the root of this directory.  
The resulting binary will appear in `target/release/plum_chess_X.X.X_YYY.exe`.   You may wish to copy this file to another more permanent place on your computer.

# How to play with Pychess
A popular front-end GUI for custom chess engines is pychess.  Pychess can be installed from the [website](https://pychess.github.io/download/).

Launch Pychess.  If this is the first time,  click your way through the splash screen pop-ups.

Select `edit >> engines >> new` then navigate to the plum_chess executable.  Pychess will then query the executable and determine it uses the `uci` interface.
Fill in the engine metadata as you see fit and then click save.

Return to the main screen.  In the top left corner selection panel you can select your color and an engine for your opponent and determine the difficulty level.  Plum chess 2.0 only has levels 0 - 7.
If you select the weather icon, you can adjust other game parameters like time limits.  Plum Chess 2.0 is not optimized around time limits and will simply waste time.

Levels 0 - Full random move generation
Levels 1 - Greedy (always attatck)
Levels 2 - 7) Iterative deepening with conventional material metric to more and more layers.

Have fun!
