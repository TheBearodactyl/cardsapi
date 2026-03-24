use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::games::{create_deck, shuffle_deck};
use crate::models::*;

pub struct WarGame {
    pub decks: HashMap<Uuid, Vec<Card>>,
    pub current_battle: HashMap<Uuid, Card>,
    pub flipped_this_round: HashMap<Uuid, bool>,
    pub war_pile: Vec<Card>,
    pub phase: WarPhase,
    pub player_order: Vec<Uuid>,
    pub last_result: Option<String>,
}

impl WarGame {
    pub fn new(player_ids: &[Uuid]) -> Self {
        assert_eq!(player_ids.len(), 2);

        let mut deck = create_deck();
        shuffle_deck(&mut deck);

        let half = deck.len() / 2;
        let mut decks = HashMap::new();
        decks.insert(player_ids[0], deck[..half].to_vec());
        decks.insert(player_ids[1], deck[half..].to_vec());

        WarGame {
            decks,
            current_battle: HashMap::new(),
            flipped_this_round: HashMap::new(),
            war_pile: Vec::new(),
            phase: WarPhase::Flip,
            player_order: player_ids.to_vec(),
            last_result: None,
        }
    }

    pub fn process_action(
        &mut self,
        player_id: Uuid,
        action: GameAction,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        if self.phase == WarPhase::Finished {
            return Err(ApiError::InvalidAction("Game is already over".into()));
        }

        match action {
            GameAction::FlipCard => self.handle_flip(player_id, player_names),
            _ => Err(ApiError::InvalidAction("Invalid action for War".into())),
        }
    }

    fn handle_flip(
        &mut self,
        player_id: Uuid,
        player_names: &HashMap<Uuid, String>,
    ) -> Result<ActionResult, ApiError> {
        if self.flipped_this_round.contains_key(&player_id) {
            return Err(ApiError::NotYourTurn);
        }

        let player_deck = self
            .decks
            .get_mut(&player_id)
            .ok_or(ApiError::InvalidAction("Player not in game".into()))?;

        if player_deck.is_empty() {
            return Err(ApiError::InvalidAction("No cards left to flip".into()));
        }

        let card = player_deck.remove(0);
        self.current_battle.insert(player_id, card.clone());
        self.flipped_this_round.insert(player_id, true);

        let player_name = player_names
            .get(&player_id)
            .cloned()
            .unwrap_or_default();

        
        if self.flipped_this_round.len() == 2 {
            let result = self.resolve_battle(player_names);
            return Ok(result);
        }

        
        Ok(ActionResult {
            description: format!("{} flipped a card", player_name),
            action_name: "FLIP_CARD".into(),
            result_label: Some("WAITING".into()),
            details: serde_json::json!({
                "card": card
            }),
            game_over: None,
        })
    }

    fn resolve_battle(&mut self, player_names: &HashMap<Uuid, String>) -> ActionResult {
        let p1 = self.player_order[0];
        let p2 = self.player_order[1];
        let c1 = self.current_battle[&p1].clone();
        let c2 = self.current_battle[&p2].clone();

        let p1_name = player_names.get(&p1).cloned().unwrap_or_default();
        let p2_name = player_names.get(&p2).cloned().unwrap_or_default();

        let v1 = c1.rank.war_value();
        let v2 = c2.rank.war_value();

        let mut battle_cards: Vec<Card> = self.current_battle.values().cloned().collect();
        battle_cards.extend(self.war_pile.drain(..));

        if v1 == v2 {
            
            self.war_pile.extend(battle_cards);

            
            for &pid in &[p1, p2] {
                let deck = self.decks.get_mut(&pid).unwrap();
                let take = std::cmp::min(3, deck.len());
                let face_down: Vec<Card> = deck.drain(..take).collect();
                self.war_pile.extend(face_down);

                if deck.is_empty() {
                    
                    return self.finish_with_winner(
                        if pid == p1 { p2 } else { p1 },
                        player_names,
                        format!(
                            "{} ran out of cards during War!",
                            player_names.get(&pid).cloned().unwrap_or_default()
                        ),
                    );
                }
            }

            self.phase = WarPhase::War;
            self.current_battle.clear();
            self.flipped_this_round.clear();

            let description =
                format!("War! {} vs {} - both played {}", p1_name, p2_name, c1.rank.display_name());
            self.last_result = Some(description.clone());

            ActionResult {
                description,
                action_name: "FLIP_CARD".into(),
                result_label: Some("WAR".into()),
                details: serde_json::json!({
                    "warPileCount": self.war_pile.len()
                }),
                game_over: None,
            }
        } else {
            let (winner_id, winner_name, loser_card) = if v1 > v2 {
                (p1, &p1_name, &c2)
            } else {
                (p2, &p2_name, &c1)
            };

            let winner_card = &self.current_battle[&winner_id];
            let description = format!(
                "{} wins the battle ({} vs {})",
                winner_name,
                winner_card.rank.display_name(),
                loser_card.rank.display_name()
            );

            
            self.decks
                .get_mut(&winner_id)
                .unwrap()
                .extend(battle_cards);

            self.current_battle.clear();
            self.flipped_this_round.clear();
            self.phase = WarPhase::Flip;
            self.last_result = Some(description.clone());

            
            let game_over = self.check_game_over(player_names);

            ActionResult {
                description,
                action_name: "FLIP_CARD".into(),
                result_label: Some("BATTLE_WON".into()),
                details: serde_json::json!({
                    "winnerId": winner_id
                }),
                game_over,
            }
        }
    }

    fn finish_with_winner(
        &mut self,
        winner_id: Uuid,
        player_names: &HashMap<Uuid, String>,
        reason: String,
    ) -> ActionResult {
        self.phase = WarPhase::Finished;
        self.current_battle.clear();
        self.flipped_this_round.clear();
        self.last_result = Some(reason.clone());

        let winner_name = player_names.get(&winner_id).cloned();

        let final_scores: HashMap<String, u32> = self
            .player_order
            .iter()
            .map(|&pid| (pid.to_string(), self.decks.get(&pid).map(|d| d.len() as u32).unwrap_or(0)))
            .collect();

        ActionResult {
            description: reason.clone(),
            action_name: "FLIP_CARD".into(),
            result_label: Some("GAME_OVER".into()),
            details: serde_json::json!({}),
            game_over: Some(GameOverInfo {
                winner_id: Some(winner_id),
                winner_name,
                final_scores,
                reason,
            }),
        }
    }

    fn check_game_over(&mut self, player_names: &HashMap<Uuid, String>) -> Option<GameOverInfo> {
        for &pid in &self.player_order {
            if self.decks.get(&pid).map(|d| d.is_empty()).unwrap_or(true) {
                let winner_id = self
                    .player_order
                    .iter()
                    .find(|&&id| id != pid)
                    .copied()
                    .unwrap();
                self.phase = WarPhase::Finished;

                let winner_name = player_names.get(&winner_id).cloned();
                let loser_name = player_names
                    .get(&pid)
                    .cloned()
                    .unwrap_or_default();

                let final_scores: HashMap<String, u32> = self
                    .player_order
                    .iter()
                    .map(|&id| {
                        (
                            id.to_string(),
                            self.decks.get(&id).map(|d| d.len() as u32).unwrap_or(0),
                        )
                    })
                    .collect();

                let reason = format!("{} has no cards remaining", loser_name);
                self.last_result = Some(reason.clone());

                return Some(GameOverInfo {
                    winner_id: Some(winner_id),
                    winner_name,
                    final_scores,
                    reason,
                });
            }
        }
        None
    }

    pub fn view_for_player(
        &self,
        player_names: &HashMap<Uuid, String>,
    ) -> WarStateView {
        let players: Vec<WarPlayerView> = self
            .player_order
            .iter()
            .map(|&pid| WarPlayerView {
                player_id: pid,
                name: player_names.get(&pid).cloned().unwrap_or_default(),
                card_count: self.decks.get(&pid).map(|d| d.len()).unwrap_or(0),
            })
            .collect();

        let current_battle = if self.current_battle.is_empty() {
            None
        } else {
            let map: HashMap<String, Card> = self
                .current_battle
                .iter()
                .map(|(id, card)| (id.to_string(), card.clone()))
                .collect();
            Some(map)
        };

        WarStateView {
            players,
            current_battle,
            war_pile_count: self.war_pile.len(),
            phase: self.phase,
            last_result: self.last_result.clone(),
        }
    }
}
