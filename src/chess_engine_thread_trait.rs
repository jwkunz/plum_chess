use crate::{chess_move::ChessMove, game_state::GameState};

/// Trait describing the minimal engine thread interface expected by the UCI handler.
///
/// Implementors encapsulate an engine instance that can be prepared with a position,
/// started and stopped for a search, and queried for completion and the best move.
/// The trait is intentionally small to allow both concrete and boxed dynamic usage:
/// - `new` is constrained to `Sized` so the trait remains object-safe for the other methods.
/// - All other methods use `&self`/`&mut self` without generic parameters.
///
/// Typical usage:
/// - Construct a concrete implementor (e.g., `EngineRandom::new()`).
/// - Call `setup` with a `GameState` and desired calculation time.
/// - Call `start_searching` (may be synchronous or spawn a background thread).
/// - Periodically call `is_done_searching` or let the UCI handler poll the engine.
/// - Retrieve the result with `get_best_move`.
pub trait ChessEngineThreadTrait {
    /// Create a new, uninitialized engine instance.
    ///
    /// Note: the `where Self: Sized` bound allows implementors to provide a
    /// constructor while keeping the trait object-safe for the other methods.
    /// Concrete types should initialize internal fields but need not start any threads.
    fn new() -> Self where Self: Sized;

    /// Prepare the engine for a search.
    ///
    /// - `game`: the position that should be searched (the engine may clone and store it).
    /// - `calculation_time_s`: the requested calculation time in seconds (engines may ignore or adapt).
    ///
    /// This call should reset any prior search state so a subsequent `start_searching`
    /// performs a fresh calculation for the provided position.
    fn setup(&mut self, game: &GameState, calculation_time_s: f32);

    /// Begin (or schedule) the search for the best move.
    ///
    /// Implementations may perform searching synchronously or spawn background work.
    /// The UCI handler expects `is_done_searching` and `get_best_move` to reflect progress/results.
    fn start_searching(&mut self);

    /// Request the engine to stop searching as soon as practical.
    ///
    /// Engines should make a best effort to stop promptly and make partial results
    /// available via `get_best_move` when appropriate.
    fn stop_searching(&mut self);

    /// Query whether the current search is finished.
    ///
    /// Returns `true` when the engine has completed its search and any best-move
    /// result is available via `get_best_move`.
    fn is_done_searching(&self) -> bool;

    /// Obtain the best move found by the last completed search, if any.
    ///
    /// Returns `Some(ChessMove)` when a best move is available, or `None` if no move
    /// was selected (for example, in a mate/stalemate/no-legal-move situation or
    /// if setup/start_searching were not called).
    fn get_best_move(&self) -> Option<ChessMove>;
}