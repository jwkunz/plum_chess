use crate::engines::engine_trait::{Engine, EngineOutput, GoParams};
use crate::game_state::game_state::GameState;
use crate::move_generation::legal_move_generator::LegalMoveGenerator;
use crate::search::board_scoring::MaterialScorer;
use crate::search::iterative_deepening::{iterative_deepening_search, SearchConfig};

pub struct IterativeEngine {
    default_depth: u8,
    move_generator: LegalMoveGenerator,
    scorer: MaterialScorer,
}

impl IterativeEngine {
    pub fn new(default_depth: u8) -> Self {
        Self {
            default_depth,
            move_generator: LegalMoveGenerator,
            scorer: MaterialScorer,
        }
    }
}

impl Engine for IterativeEngine {
    fn name(&self) -> &str {
        "PlumChess Iterative"
    }

    fn author(&self) -> &str {
        "jwkunz+codex"
    }

    fn choose_move(
        &mut self,
        game_state: &GameState,
        params: &GoParams,
    ) -> Result<EngineOutput, String> {
        let depth = self.default_depth.max(1);

        let result = iterative_deepening_search(
            game_state,
            &self.move_generator,
            &self.scorer,
            SearchConfig { max_depth: depth },
        )
        .map_err(|e| e.to_string())?;

        let mut out = EngineOutput::default();
        out.best_move = result.best_move;
        out.info_lines.push(format!(
            "info depth {} score cp {} nodes {}",
            result.reached_depth, result.best_score, result.nodes
        ));
        out.info_lines.push(format!(
            "info string iterative_engine default_depth {}",
            self.default_depth
        ));
        if let Some(requested) = params.depth {
            out.info_lines.push(format!(
                "info string iterative_engine ignoring_go_depth {}",
                requested
            ));
        }

        Ok(out)
    }
}
