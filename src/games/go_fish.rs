use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::games::{create_deck, shuffle_deck};
use crate::models::*;

pub struct GoFishGame {
    pub hands: HashMap<Uuid, Vec<Card>>,
    pub pairs: HashMap<Uuid, u32>,
    pub deck: Vec<Card>,
    pub current_turn_index: usize,
    pub player_order: Vec<Uuid>,
    pub last_action: Option<String>,
    pub finished: bool,
}

impl GoFishGame {
    pub fn new(player_ids: &[Uuid]) -> Self {
        let mut deck = create_deck();
        shuffle_deck(&mut deck);

        let cards_per_player = if player_ids.len() <= 2 { 7 } else { 5 };

        let mut hands = HashMap::new();
        let mut pairs = HashMap::new();

        for &pid in player_ids {
            let hand: Vec<Card> = deck.drain(..cards_per_player).collect();
            hands.insert(pid, hand);
            pairs.insert(pid, 0);
        }

        let mut game = GoFishGame {
            hands,
            pairs,
            deck,
            current_turn_index: 0,
            player_order: player_ids.to_vec(),
            last_action: None,
            finished: false,
        };

        
        for &pid in player_ids {
            game.lay_down_pairs(pid);
        }

        game
    }

    fn lay_down_pairs(&mut self, player_id: Uuid) {
        let hand = self.hands.get_mut(&player_id).unwrap();
        let mut rank_counts: HashMap<Rank, usize> = HashMap::new();
        for card in hand.iter() {
            *rank_counts.entry(card.rank).or_insert(0) += 1;
        }

        for (rank, count) in &rank_counts {
            if *count >= 2 {
                let pairs_to_remove = count / 2;
                let cards_to_remove = pairs_to_remove * 2;
                let mut removed = 0;
                hand.retain(|c| {
                    if c.rank == *rank && removed < cards_to_remove {
                        removed += 1;
                        false
                    } else {
                        true
                    }
                });
                *self.pairs.get_mut(&player_id).unwrap() += pairs_to_remove as u32;
            }
        }
    }

    fn current_turn_player(&self) -> Uuid {
        self.player_order[self.current_turn_index]
    }

    fn advance_turn(&mut self) {
        self.current_turn_index = (self.current_turn_index + 1) % self.player_order.len();
        self.refill_empty_hand(self.current_turn_player());
    }

    fn refill_empty_hand(&mut self, player_id: Uuid) {
        let hand = self.hands.get(&player_id).unwrap();
        if hand.is_empty() && !self.deck.is_empty() {
            let card = self.deck.pop().unwrap();
            self.hands.get_mut(&player_id).unwrap().push(card);
            self.lay_down_pairs(player_id);
        }
    }

    fn check_game_over(&mut self) {
        
        let total_pairs: u32 = self.pairs.values().sum();
        if total_pairs == 26 {
            self.finished = true;
            return;
        }

        
        if self.deck.is_empty() && self.hands.values().all(|h| h.is_empty()) {
            self.finished = true;
        }
    }

    pub fn process_action(
        &mut self,
        player_id: Uuid,
        action: GameAction,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        if self.finished {
            return Err(ApiError::InvalidAction("Game is already over".into()));
        }

        let current = self.current_turn_player();
        if player_id != current {
            return Err(ApiError::NotYourTurn);
        }

        match action {
            GameAction::AskForCard {
                target_player_id,
                rank,
            } => self.handle_ask(player_id, target_player_id, rank, player_names),
            _ => Err(ApiError::InvalidAction(
                "Invalid action for Go Fish".into(),
            )),
        }
    }

    fn handle_ask(
        &mut self,
        player_id: Uuid,
        target_id: Uuid,
        rank: Rank,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        if target_id == player_id {
            return Err(ApiError::InvalidAction("Cannot ask yourself".into()));
        }

        if !self.hands.contains_key(&target_id) {
            return Err(ApiError::InvalidAction("Target player not found".into()));
        }

        
        let has_rank = self
            .hands
            .get(&player_id)
            .unwrap()
            .iter()
            .any(|c| c.rank == rank);
        if !has_rank {
            return Err(ApiError::InvalidAction(format!(
                "You must hold at least one {} to ask for it",
                rank.display_name()
            )));
        }

        let player_name = player_names
            .get(&player_id)
            .cloned()
            .unwrap_or_default();
        let target_name = player_names
            .get(&target_id)
            .cloned()
            .unwrap_or_default();

        
        let target_hand = self.hands.get_mut(&target_id).unwrap();
        let matching: Vec<Card> = target_hand
            .iter()
            .filter(|c| c.rank == rank)
            .cloned()
            .collect();
        let cards_transferred = matching.len();

        let (result_label, description) = if cards_transferred > 0 {
            
            target_hand.retain(|c| c.rank != rank);
            self.hands
                .get_mut(&player_id)
                .unwrap()
                .extend(matching);
            self.lay_down_pairs(player_id);

            let desc = format!(
                "{} asked {} for {} and got {} card{}!",
                player_name,
                target_name,
                rank.display_plural(),
                cards_transferred,
                if cards_transferred == 1 { "" } else { "s" }
            );
            
            ("SUCCESS".to_string(), desc)
        } else {
            
            let mut drew_match = false;
            if let Some(drawn) = self.deck.pop() {
                if drawn.rank == rank {
                    drew_match = true;
                }
                self.hands.get_mut(&player_id).unwrap().push(drawn);
                self.lay_down_pairs(player_id);
            }

            let desc = format!(
                "{} asked {} for {}. Go Fish!",
                player_name, target_name, rank.display_plural()
            );

            if !drew_match {
                self.advance_turn();
            }
            

            ("GO_FISH".to_string(), desc)
        };

        self.last_action = Some(description.clone());
        self.check_game_over();

        let game_over = if self.finished {
            Some(self.build_game_over(player_names))
        } else {
            None
        };

        Ok(ActionResult {
            description,
            action_name: "ASK_FOR_CARD".into(),
            result_label: Some(result_label),
            details: serde_json::json!({
                "targetPlayerName": target_name,
                "rank": rank,
                "cardsTransferred": cards_transferred
            }),
            game_over,
        })
    }

    fn build_game_over(&self, player_names: &HashMap<Uuid, String>) -> GameOverInfo {
        let mut max_pairs = 0u32;
        let mut winner_id = None;

        let final_scores: HashMap<String, u32> = self
            .pairs
            .iter()
            .map(|(id, &p)| {
                if p > max_pairs {
                    max_pairs = p;
                    winner_id = Some(*id);
                }
                (id.to_string(), p)
            })
            .collect();

        let winner_name = winner_id
            .and_then(|id| player_names.get(&id))
            .cloned();

        GameOverInfo {
            winner_id,
            winner_name: winner_name.clone(),
            final_scores,
            reason: format!(
                "{} collected the most pairs",
                winner_name.unwrap_or_default()
            ),
        }
    }

    pub fn view_for_player(
        &self,
        player_id: Uuid,
        player_names: &HashMap<Uuid, String>,
    ) -> GoFishStateView {
        let hand = self
            .hands
            .get(&player_id)
            .cloned()
            .unwrap_or_default();

        let opponents: Vec<GoFishOpponent> = self
            .player_order
            .iter()
            .filter(|&&pid| pid != player_id)
            .map(|&pid| GoFishOpponent {
                player_id: pid,
                name: player_names.get(&pid).cloned().unwrap_or_default(),
                card_count: self.hands.get(&pid).map(|h| h.len()).unwrap_or(0),
            })
            .collect();

        let pairs: HashMap<String, u32> = self
            .pairs
            .iter()
            .map(|(id, &p)| (id.to_string(), p))
            .collect();

        GoFishStateView {
            hand,
            opponents,
            current_turn: self.current_turn_player(),
            pairs,
            last_action: self.last_action.clone(),
            deck_remaining: self.deck.len(),
        }
    }
}
