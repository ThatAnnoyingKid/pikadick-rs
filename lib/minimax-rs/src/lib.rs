/// Built-in tic-tac-toe support
///
pub mod tic_tac_toe;

pub use self::tic_tac_toe::{
    TicTacToeRuleSet,
    TicTacToeState,
    TicTacToeTeam,
};
use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
};

/// An abstract set of rules for a game.
///
pub trait RuleSet {
    /// The game state
    ///
    type State: Debug + Eq + Hash + Clone;

    /// The game teams
    ///
    type Team: Debug + Eq + Hash + Clone;

    /// Get the starting node for minimax-ing.
    ///
    fn get_start_state() -> Self::State;

    /// Get the team from a [`Self::State`].
    ///
    fn get_team(state: &Self::State) -> Self::Team;

    /// Get the winner for a [`Self::State`].
    ///
    /// Returns `None` if there is no winner yet.
    ///
    fn get_winner(state: &Self::State) -> Option<Self::Team>;

    /// Get the child states for a given [`Self::State`].
    ///
    fn get_child_states(state: &Self::State) -> Vec<Self::State>;

    /// Score a node's score based on a winner
    ///
    fn score_winner(winner: &Self::Team, score: &mut i8);

    /// Score a state.
    ///
    fn score_state(state: &Self::State, child_scores: &[i8]) -> i8;

    /// Choose the best state for a given state and team.
    ///
    fn choose_best_state<'a>(
        state: &'a Self::State,
        score: i8,
        best_state: &mut &'a Self::State,
        best_score: i8,
        team: &Self::Team,
    );
}

/// Compile a minimax map with a [`RuleSet`].
///
pub fn compile_minimax_map<R>() -> HashMap<R::State, Node<R::State>>
where
    R: RuleSet,
{
    let mut node_map = HashMap::new();
    let mut processing_queue = VecDeque::new();
    let mut unscored_states = VecDeque::new();

    let start_node = Node::from_state(R::get_start_state());
    node_map.insert(start_node.state.clone(), start_node.clone());
    processing_queue.push_back(start_node.state);

    while let Some(state) = processing_queue.pop_front() {
        if R::get_winner(&state).is_some() {
            continue;
        }

        unscored_states.push_back(state.clone());

        let states = R::get_child_states(&state);

        for child_state in states {
            if !node_map.contains_key(&child_state) {
                let child_node = {
                    let mut node = Node::from_state(child_state.clone());

                    if let Some(winner) = R::get_winner(&child_state) {
                        R::score_winner(&winner, &mut node.score);
                    }

                    node
                };
                node_map.insert(child_state.clone(), child_node);
                processing_queue.push_back(child_state.clone());
            }

            node_map
                .get_mut(&child_state)
                .expect("missing child node")
                .parents
                .push(state.clone());

            node_map
                .get_mut(&state)
                .expect("missing parent node")
                .children
                .push(child_state.clone());
        }
    }

    // TODO: This is too reliant on order. Make algo work from winning nodes up.
    while let Some(state) = unscored_states.pop_back() {
        let children = &node_map
            .get(&state)
            .expect("missing parent node state")
            .children;
        let mut scores = Vec::with_capacity(children.len());

        for child_state in children.iter() {
            scores.push(
                node_map
                    .get(&child_state)
                    .expect("missing child node state")
                    .score,
            );
        }

        let score = R::score_state(&state, &scores);

        let mut node = node_map.get_mut(&state).expect("missing node state");
        if node.score != 0 {
            panic!("node score already present");
        }
        node.score = score;
    }

    node_map
}

/// A Node in a game graph.
///
#[derive(Debug, Clone)]
pub struct Node<S> {
    state: S,

    parents: Vec<S>,
    children: Vec<S>,

    score: i8,
}

impl<S> Node<S> {
    /// Make a new [`Node`] from a `State`.
    ///
    pub fn from_state(state: S) -> Self {
        Self {
            state,
            parents: Vec::new(),
            children: Vec::new(),

            score: 0,
        }
    }
}

/// A type that can use a compiled minimax map.
///
pub struct MiniMaxAi<R>
where
    R: RuleSet,
{
    map: HashMap<R::State, Node<R::State>>,

    _rule_set: PhantomData<R>,
}

impl<R> MiniMaxAi<R>
where
    R: RuleSet,
{
    /// Make a new [`MiniMaxAi`].
    ///
    pub fn new(map: HashMap<R::State, Node<R::State>>) -> Self {
        Self {
            map,
            _rule_set: PhantomData,
        }
    }

    /// Get a [`Node`] from a game state.
    ///
    pub fn get_node(&self, state: &R::State) -> Option<&Node<R::State>> {
        self.map.get(state)
    }

    /// Get a move for a given team and state.
    ///
    pub fn get_move(&self, state: &R::State, team: &R::Team) -> Option<&R::State> {
        let node = self.get_node(&state)?;

        let mut best_child_state = node.children.get(0)?;
        for child_state in node.children.iter() {
            let child_score = self.get_score(&child_state)?;
            let best_child_score = self.get_score(&best_child_state)?;

            R::choose_best_state(
                &child_state,
                child_score,
                &mut best_child_state,
                best_child_score,
                &team,
            );
        }

        Some(&best_child_state)
    }

    /// Get the score for a state
    ///
    pub fn get_score(&self, state: &R::State) -> Option<i8> {
        self.get_node(state).map(|node| node.score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let map = compile_minimax_map::<TicTacToeRuleSet>();
        dbg!(map.len());

        let ai: MiniMaxAi<TicTacToeRuleSet> = MiniMaxAi::new(map);

        dbg!(ai.get_move(&TicTacToeState::default(), &TicTacToeTeam::X));
    }
}
