/// Represents the team (color) of a chess piece.
/// Used to distinguish between dark (black) and light (white) pieces.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PieceTeam {
    /// The dark (black) side.
    Dark,
    /// The light (white) side.
    Light,
}
