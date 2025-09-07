#[derive(Debug)]
pub enum Errors {
    OutOfBounds,
    RuntimeError,
    GameRuleError,
    BoardLocationOccupied,
    InvalidFENstring,
    InvalidAlgebraic,
    InvalidMoveStartCondition,
}
