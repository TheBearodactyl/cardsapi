use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::games::{create_deck, shuffle_deck};
use crate::models::*;

pub struct BlackjackGame {
    pub player_hands: HashMap<Uuid, Vec<Card>>,
    pub player_statuses: HashMap<Uuid, BlackjackPlayerStatus>,
    pub player_results: HashMap<Uuid, BlackjackResult>,
    pub dealer_hand: Vec<Card>,
    pub deck: Vec<Card>,
    pub current_turn_index: usize,
    pub player_order: Vec<Uuid>,
    pub phase: BlackjackPhase,
}

fn calculate_score(hand: &[Card]) -> u32 {
    let mut score: u32 = 0;
    let mut aces: u32 = 0;
    for card in hand {
        if card.rank == Rank::Ace {
            score += 11;
            aces += 1;
        } else {
            score += card.rank.blackjack_value();
        }
    }
    while score > 21 && aces > 0 {
        score -= 10;
        aces -= 1;
    }
    score
}

impl BlackjackGame {
    pub fn new(player_ids: &[Uuid]) -> Self {
        let mut deck = create_deck();
        shuffle_deck(&mut deck);

        let mut player_hands = HashMap::new();
        let mut player_statuses = HashMap::new();

        
        for &pid in player_ids {
            let hand = vec![deck.pop().unwrap(), deck.pop().unwrap()];
            player_hands.insert(pid, hand);
            player_statuses.insert(pid, BlackjackPlayerStatus::Playing);
        }

        
        let dealer_hand = vec![deck.pop().unwrap(), deck.pop().unwrap()];

        let mut game = BlackjackGame {
            player_hands,
            player_statuses,
            player_results: HashMap::new(),
            dealer_hand,
            deck,
            current_turn_index: 0,
            player_order: player_ids.to_vec(),
            phase: BlackjackPhase::PlayerTurns,
        };

        
        for &pid in player_ids {
            let score = calculate_score(game.player_hands.get(&pid).unwrap());
            if score == 21 {
                game.player_statuses
                    .insert(pid, BlackjackPlayerStatus::Blackjack);
            }
        }

        
        game.skip_non_playing();

        game
    }

    fn current_turn_player(&self) -> Option<Uuid> {
        if self.phase != BlackjackPhase::PlayerTurns {
            return None;
        }
        if self.current_turn_index >= self.player_order.len() {
            return None;
        }
        Some(self.player_order[self.current_turn_index])
    }

    fn skip_non_playing(&mut self) {
        while self.current_turn_index < self.player_order.len() {
            let pid = self.player_order[self.current_turn_index];
            if self.player_statuses[&pid] == BlackjackPlayerStatus::Playing {
                return;
            }
            self.current_turn_index += 1;
        }
        
        if self.phase == BlackjackPhase::PlayerTurns {
            self.play_dealer();
        }
    }

    fn advance_turn(&mut self) {
        self.current_turn_index += 1;
        self.skip_non_playing();
    }

    fn play_dealer(&mut self) {
        self.phase = BlackjackPhase::DealerTurn;

        
        let all_busted = self
            .player_order
            .iter()
            .all(|pid| self.player_statuses[pid] == BlackjackPlayerStatus::Busted);

        if !all_busted {
            
            while calculate_score(&self.dealer_hand) < 17 {
                if let Some(card) = self.deck.pop() {
                    self.dealer_hand.push(card);
                } else {
                    break;
                }
            }
        }

        self.finish_game();
    }

    fn finish_game(&mut self) {
        self.phase = BlackjackPhase::Finished;
        let dealer_score = calculate_score(&self.dealer_hand);
        let dealer_busted = dealer_score > 21;

        for &pid in &self.player_order {
            let status = self.player_statuses[&pid];
            let player_score = calculate_score(self.player_hands.get(&pid).unwrap());

            let result = match status {
                BlackjackPlayerStatus::Busted => BlackjackResult::Lose,
                BlackjackPlayerStatus::Blackjack => {
                    if dealer_score == 21 && self.dealer_hand.len() == 2 {
                        BlackjackResult::Push 
                    } else {
                        BlackjackResult::Win
                    }
                }
                _ => {
                    if dealer_busted {
                        BlackjackResult::Win
                    } else if player_score > dealer_score {
                        BlackjackResult::Win
                    } else if player_score == dealer_score {
                        BlackjackResult::Push
                    } else {
                        BlackjackResult::Lose
                    }
                }
            };

            self.player_results.insert(pid, result);
        }
    }

    pub fn process_action(
        &mut self,
        player_id: Uuid,
        action: GameAction,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        if self.phase != BlackjackPhase::PlayerTurns {
            return Err(ApiError::InvalidAction(
                "Not in player turns phase".into(),
            ));
        }

        let current = self
            .current_turn_player()
            .ok_or(ApiError::InvalidAction("No active turn".into()))?;
        if player_id != current {
            return Err(ApiError::NotYourTurn);
        }

        let player_name = player_names
            .get(&player_id)
            .cloned()
            .unwrap_or_default();

        match action {
            GameAction::Hit => self.handle_hit(player_id, &player_name, player_names),
            GameAction::Stand => self.handle_stand(player_id, &player_name, player_names),
            _ => Err(ApiError::InvalidAction(
                "Invalid action for Blackjack".into(),
            )),
        }
    }

    fn handle_hit(
        &mut self,
        player_id: Uuid,
        player_name: &str,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        let card = self
            .deck
            .pop()
            .ok_or(ApiError::InvalidAction("Deck is empty".into()))?;

        self.player_hands
            .get_mut(&player_id)
            .unwrap()
            .push(card.clone());

        let score = calculate_score(self.player_hands.get(&player_id).unwrap());
        let busted = score > 21;

        if busted {
            self.player_statuses
                .insert(player_id, BlackjackPlayerStatus::Busted);
            self.advance_turn();
        } else if score == 21 {
            self.player_statuses
                .insert(player_id, BlackjackPlayerStatus::Stood);
            self.advance_turn();
        }

        let description = if busted {
            format!("{} hits and busts with {}", player_name, score)
        } else {
            format!("{} hits (score: {})", player_name, score)
        };

        let game_over = if self.phase == BlackjackPhase::Finished {
            Some(self.build_game_over(player_names))
        } else {
            None
        };

        Ok(ActionResult {
            description,
            action_name: "HIT".into(),
            result_label: if busted {
                Some("BUSTED".into())
            } else {
                Some("OK".into())
            },
            details: serde_json::json!({
                "card": card,
                "score": score,
                "busted": busted
            }),
            game_over,
        })
    }

    fn handle_stand(
        &mut self,
        player_id: Uuid,
        player_name: &str,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        let score = calculate_score(self.player_hands.get(&player_id).unwrap());
        self.player_statuses
            .insert(player_id, BlackjackPlayerStatus::Stood);
        self.advance_turn();

        let description = format!("{} stands with {}", player_name, score);

        let game_over = if self.phase == BlackjackPhase::Finished {
            Some(self.build_game_over(player_names))
        } else {
            None
        };

        Ok(ActionResult {
            description,
            action_name: "STAND".into(),
            result_label: Some("STOOD".into()),
            details: serde_json::json!({ "score": score }),
            game_over,
        })
    }

    fn build_game_over(&self, player_names: &HashMap<Uuid, String>) -> GameOverInfo {
        let mut final_scores: HashMap<String, u32> = HashMap::new();
        let mut winner_id: Option<Uuid> = None;
        let mut best_score = 0u32;

        for &pid in &self.player_order {
            let score = calculate_score(self.player_hands.get(&pid).unwrap());
            final_scores.insert(pid.to_string(), score);

            if let Some(&result) = self.player_results.get(&pid) {
                if result == BlackjackResult::Win && score > best_score {
                    best_score = score;
                    winner_id = Some(pid);
                }
            }
        }

        let dealer_score = calculate_score(&self.dealer_hand);
        final_scores.insert("dealer".into(), dealer_score);

        let winner_name = winner_id.and_then(|id| player_names.get(&id)).cloned();

        let reason = if let Some(ref name) = winner_name {
            format!("{} wins!", name)
        } else {
            "Dealer wins!".into()
        };

        GameOverInfo {
            winner_id,
            winner_name,
            final_scores,
            reason,
        }
    }

    pub fn view_for_player(
        &self,
        player_names: &HashMap<Uuid, String>,
    ) -> BlackjackStateView {
        let show_dealer = matches!(
            self.phase,
            BlackjackPhase::DealerTurn | BlackjackPhase::Finished
        );

        let players: Vec<BlackjackPlayerView> = self
            .player_order
            .iter()
            .map(|&pid| {
                let hand = self.player_hands.get(&pid).cloned().unwrap_or_default();
                let score = calculate_score(&hand);
                BlackjackPlayerView {
                    player_id: pid,
                    name: player_names.get(&pid).cloned().unwrap_or_default(),
                    hand,
                    score,
                    status: self.player_statuses[&pid],
                    result: self.player_results.get(&pid).copied(),
                }
            })
            .collect();

        let dealer_hand: Vec<Option<Card>> = if show_dealer {
            self.dealer_hand.iter().map(|c| Some(c.clone())).collect()
        } else {
            let mut dh = Vec::new();
            if let Some(first) = self.dealer_hand.first() {
                dh.push(Some(first.clone()));
            }
            for _ in 1..self.dealer_hand.len() {
                dh.push(None);
            }
            dh
        };

        let dealer_score = if show_dealer {
            Some(calculate_score(&self.dealer_hand))
        } else {
            None
        };

        BlackjackStateView {
            players,
            dealer_hand,
            dealer_score,
            current_turn: self.current_turn_player(),
            phase: self.phase,
        }
    }
}
