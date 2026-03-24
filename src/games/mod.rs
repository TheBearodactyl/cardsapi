pub mod blackjack;
pub mod go_fish;
pub mod war;

use crate::errors::ApiError;
use crate::models::*;
use uuid::Uuid;

pub fn create_deck() -> Vec<Card> {
    let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
    let ranks = [
        Rank::Ace,
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
    ];
    let mut deck = Vec::with_capacity(52);
    for &suit in &suits {
        for &rank in &ranks {
            deck.push(Card { suit, rank });
        }
    }
    deck
}

pub fn shuffle_deck(deck: &mut Vec<Card>) {
    use rand::seq::SliceRandom;
    let mut rng = rand::rng();
    deck.shuffle(&mut rng);
}

pub enum GameState {
    GoFish(go_fish::GoFishGame),
    Blackjack(blackjack::BlackjackGame),
    War(war::WarGame),
}

impl GameState {
    pub fn init(game_type: GameType, player_ids: &[Uuid]) -> Self {
        match game_type {
            GameType::GoFish => GameState::GoFish(go_fish::GoFishGame::new(player_ids)),
            GameType::Blackjack => GameState::Blackjack(blackjack::BlackjackGame::new(player_ids)),
            GameType::War => GameState::War(war::WarGame::new(player_ids)),
        }
    }

    pub fn process_action(
        &mut self,
        player_id: Uuid,
        action: GameAction,
        player_names: &std::collections::HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        match self {
            GameState::GoFish(g) => g.process_action(player_id, action, player_names),
            GameState::Blackjack(g) => g.process_action(player_id, action, player_names),
            GameState::War(g) => g.process_action(player_id, action, player_names),
        }
    }

    pub fn get_state_for_player(
        &self,
        player_id: Uuid,
        player_names: &std::collections::HashMap<Uuid, String>,
    ) -> serde_json::Value {
        match self {
            GameState::GoFish(g) => {
                serde_json::to_value(g.view_for_player(player_id, player_names)).unwrap()
            }
            GameState::Blackjack(g) => {
                serde_json::to_value(g.view_for_player(player_names)).unwrap()
            }
            GameState::War(g) => {
                serde_json::to_value(g.view_for_player(player_names)).unwrap()
            }
        }
    }

    pub fn player_order(&self) -> &[Uuid] {
        match self {
            GameState::GoFish(g) => &g.player_order,
            GameState::Blackjack(g) => &g.player_order,
            GameState::War(g) => &g.player_order,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self {
            GameState::GoFish(g) => g.finished,
            GameState::Blackjack(g) => g.phase == BlackjackPhase::Finished,
            GameState::War(g) => g.phase == WarPhase::Finished,
        }
    }
}
