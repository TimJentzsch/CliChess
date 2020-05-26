use pleco::{BitMove, Board, MoveList, Player};

/// The result of the playouts a node.
pub struct PlayoutResult {
    /// The number of wins for the white player.
    pub white_wins: u32,
    /// The number of wins for the black player.
    pub black_wins: u32,
    /// The number of draws.
    pub draws: u32,
}

impl PlayoutResult {
    pub fn get_count(&self) -> u32 {
        return self.white_wins + self.black_wins + self.draws;
    }
}

/// A node of the Monte-Carlo-Search-Tree.
pub struct TreeNode {
    /// The current state of the board.
    pub board: Board,
    /// The current playout results for this node.
    pub playout_result: PlayoutResult,
    // The moves available from this position.
    pub moves: Vec<TreeMove>,
}

impl TreeNode {
    /// Determines if the node has not been expanded yet
    pub fn is_leaf(&self) -> bool {
        return self.playout_result.get_count() == 0 || self.board.checkmate();
    }

    pub fn expand(&mut self) {
        let moves = self.board.generate_moves();

        let tree_moves: Vec<TreeMove> = moves
            .iter()
            .map(|mv| {
                let mut result_board = self.board.clone();
                result_board.apply_move(*mv);

                TreeMove {
                    mv: *mv,
                    next_node: TreeNode {
                        board: result_board,
                        playout_result: PlayoutResult {
                            white_wins: 0,
                            black_wins: 0,
                            draws: 0,
                        },
                        moves: vec![],
                    },
                }
            })
            .collect();

        self.moves = tree_moves;
    }
}

/// A possible move from a node.
pub struct TreeMove {
    /// The move resulting in the next node.
    pub mv: BitMove,
    /// The node resulting from the move.
    pub next_node: TreeNode,
}
