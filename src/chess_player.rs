use pleco::{BitMove, Board, MoveList, Player};
use rand::{self, rngs::ThreadRng, Rng};
use std::cmp::Ordering;
use std::io;
use std::io::BufRead;
use std::sync::mpsc::channel;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

pub trait ChessPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove;
}

pub struct HumanPlayer {}

impl HumanPlayer {
    pub fn new() -> HumanPlayer {
        HumanPlayer {}
    }
}

impl ChessPlayer for HumanPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let stdin = io::stdin();

        loop {
            let uci_move = stdin.lock().lines().next().unwrap().unwrap();

            let all_moves: MoveList = board.generate_moves();
            let bit_move: Option<BitMove> = all_moves
                .iter()
                .find(|m| m.stringify() == uci_move)
                .cloned();
            if let Some(mov) = bit_move {
                return mov;
            } else {
                println!("Invalid move. Try again:");
            }
        }
    }
}

pub struct RandomPlayer {
    rng: ThreadRng,
}

impl RandomPlayer {
    pub fn new() -> RandomPlayer {
        let rng = rand::thread_rng();
        RandomPlayer { rng: rng }
    }
}

impl ChessPlayer for RandomPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let all_moves: MoveList = board.generate_moves();
        let rnd = self.rng.gen_range(0 as usize, all_moves.len());

        all_moves[rnd]
    }
}

enum Playout {
    Win,
    Draw,
    Loss,
}

struct Node {
    board: Board,
    mv: BitMove,
    wins: u32,
    losses: u32,
    draws: u32
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Node {
            board: self.board.clone(),
            mv: self.mv.clone(),
            wins: self.wins,
            losses: self.losses,
            draws: self.draws,
        }
    }
}

impl Node {
    pub fn new(src_board: &Board, mv: BitMove) -> Node {
        let mut board = src_board.clone();
        board.apply_move(mv);
        Node {
            board: board,
            mv: mv,
            wins: 0,
            losses: 0,
            draws: 0,
        }
    }

    pub fn update(&mut self, playout: Playout) {
        match playout {
            Playout::Win => self.wins += 1,
            Playout::Loss => self.losses += 1,
            Playout::Draw => self.draws += 1
        }
    }

    pub fn mcts_value(&self, total_playouts: usize) -> f32 {
        let exploitation = self.play_value();
        let exploration = if self.playouts() == 0 {
            1.
        } else {
            let exploration_factor = 1.4142; // sqrt(2)
            exploration_factor * ((total_playouts as f32).ln() / (self.playouts() as f32)).sqrt()
        };
        exploitation + exploration
    }

    pub fn mcts_cmp(&self, other: &Node, total_playouts: usize) -> Ordering {
        match self
            .mcts_value(total_playouts)
            .partial_cmp(&other.mcts_value(total_playouts))
        {
            Option::Some(ord) => ord,
            Option::None => Ordering::Less,
        }
    }

    pub fn playouts(&self) -> u32 {
        self.wins + self.losses + self.draws
    }

    pub fn play_value(&self) -> f32 {
        if self.playouts() == 0 {
            0.5
        } else {
            self.value() / (self.playouts() as f32)
        }
    }

    pub fn value(&self) -> f32 {
        (self.wins as f32) + (self.draws as f32) * 0.5
    }

    pub fn pure_winrate(&self) -> f32 {
        let winners = self.wins + self.losses;
        if winners == 0 {
            0.5
        } else {
            self.wins as f32 / winners as f32
        }
    }

    pub fn play_cmp(&self, other: &Node) -> Ordering {
        match self.play_value().partial_cmp(&other.play_value()) {
            Option::Some(ord) => ord,
            Option::None => Ordering::Less,
        }
    }
}

pub struct StoneFish {
    player: Player,
    expand_size: usize,
    playout_size: usize,
}

impl StoneFish {
    pub fn new(player: Player, expand_size: usize, playout_size: usize) -> StoneFish {
        StoneFish {
            player: player,
            expand_size: expand_size,
            playout_size: playout_size,
        }
    }

    fn playout(mut board: Board, player: Player) -> Playout {
        loop {
            // Check for game end
            if board.rule_50() >= 50 {
                return Playout::Draw;
            }
            let moves = board.generate_moves();
            if moves.len() == 0 {
                if board.stalemate() {
                    // println!("Stalemate!");
                    return Playout::Draw;
                } else if board.checkmate() {
                    if board.turn() == player {
                        // println!("Loss!");
                        return Playout::Loss;
                    } else {
                        // println!("Win!");
                        return Playout::Win;
                    }
                } else {
                    return Playout::Draw;
                }
            }

            // Choose random move
            let mut rng = rand::thread_rng();
            let rnd = rng.gen_range(0 as usize, moves.len());
            let mv = moves[rnd];

            // Playout with that move
            board.apply_move(mv);
        }
    }

    fn parallel_playout_old(&self, node: &mut Node) {
        // Make playouts for that node
        for _ in 0..self.playout_size {
            let board_clone = node.board.clone();
            let player = self.player.clone();
            let playout = StoneFish::playout(board_clone, player);
            node.update(playout);
        }
    }

    fn parallel_playout(mut node: Node, playout_size: usize, player: Player) -> Node {
        let (tx, rx) = channel();

        // Do the playouts in parallel
        for _ in 0..playout_size {
            let tx = tx.clone();
            let board = node.board.clone();
            thread::spawn(move || {
                let playout = StoneFish::playout(board, player);
                tx.send(playout).unwrap();
            });
        }

        // Update the node
        for _ in 0..playout_size {
            let playout = rx.recv().unwrap();
            node.update(playout);
        }

        node
    }

    fn parallel_expand(
        nodes: &mut Vec<Node>,
        expand_size: usize,
        playout_size: usize,
        play_counter: usize,
        player: Player,
    ) -> usize {
        nodes.sort_by(|a, b| a.mcts_cmp(b, play_counter));
        let len = nodes.len();

        let expand_size = if expand_size > len { len } else { expand_size };

        let (tx, rx) = channel();

        for _ in 0..expand_size {
            let tx = tx.clone();
            let best_exp_node = nodes.pop().unwrap();
            thread::spawn(move || {
                let node = StoneFish::parallel_playout(best_exp_node, playout_size, player);
                tx.send(node).unwrap();
            });
        }

        for _ in 0..expand_size {
            let node = rx.recv().unwrap();
            nodes.push(node);
        }

        play_counter + expand_size * playout_size
    }
}

impl ChessPlayer for StoneFish {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let now = SystemTime::now();
        let moves = board.generate_moves();

        let mut play_counter = 0;

        // If only one move is available, make that move
        if moves.len() == 1 {
            return moves[0];
        }

        let mut nodes = Vec::<Node>::new();

        // Map moves to nodes
        for mv in &moves {
            let node = Node::new(board, mv);
            nodes.push(node);
        }

        while now.elapsed().unwrap() < time {
            play_counter = StoneFish::parallel_expand(
                &mut nodes,
                self.expand_size,
                self.playout_size,
                play_counter,
                self.player,
            );
        }

        nodes.sort_by(|a, b| a.play_cmp(b));

        let best_node = nodes.last().unwrap();

        println!(
            "{}/{} ({}) | {}/{} ({:05.1}%) | {}/{} ({:05.1}%) | {}s",
            play_counter,
            moves.len(),
            play_counter / moves.len(),
            best_node.value(),
            best_node.playouts(),
            best_node.play_value() * 100.,
            best_node.wins,
            best_node.wins + best_node.losses,
            best_node.pure_winrate() * 100.,
            now.elapsed().unwrap().as_secs()
        );

        best_node.mv
    }
}
