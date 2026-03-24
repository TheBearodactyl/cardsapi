use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;



#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GameType {
    GoFish,
    Blackjack,
    War,
}

impl GameType {
    pub fn min_players(self) -> u32 {
        match self {
            GameType::GoFish => 2,
            GameType::Blackjack => 1,
            GameType::War => 2,
        }
    }

    pub fn max_players(self) -> u32 {
        match self {
            GameType::GoFish => 6,
            GameType::Blackjack => 7,
            GameType::War => 2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoomStatus {
    Waiting,
    InProgress,
    Finished,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub fn war_value(self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }

    pub fn blackjack_value(self) -> u32 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 10,
            Rank::Ace => 11,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Rank::Ace => "Ace",
            Rank::Two => "Two",
            Rank::Three => "Three",
            Rank::Four => "Four",
            Rank::Five => "Five",
            Rank::Six => "Six",
            Rank::Seven => "Seven",
            Rank::Eight => "Eight",
            Rank::Nine => "Nine",
            Rank::Ten => "Ten",
            Rank::Jack => "Jack",
            Rank::Queen => "Queen",
            Rank::King => "King",
        }
    }

    pub fn display_plural(self) -> &'static str {
        match self {
            Rank::Six => "Sixes",
            _ => {
                
                
                match self {
                    Rank::Ace => "Aces",
                    Rank::Two => "Twos",
                    Rank::Three => "Threes",
                    Rank::Four => "Fours",
                    Rank::Five => "Fives",
                    Rank::Six => "Sixes",
                    Rank::Seven => "Sevens",
                    Rank::Eight => "Eights",
                    Rank::Nine => "Nines",
                    Rank::Ten => "Tens",
                    Rank::Jack => "Jacks",
                    Rank::Queen => "Queens",
                    Rank::King => "Kings",
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}



#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub code: String,
    pub game_type: GameType,
    pub host_id: Uuid,
    pub players: Vec<Player>,
    pub status: RoomStatus,
    pub max_players: u32,
    pub min_players: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub connected: bool,
}



#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoomRequest {
    pub game_type: GameType,
}

#[derive(Deserialize)]
pub struct UpdateNameRequest {
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}



#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GameAction {
    #[serde(rename = "ASK_FOR_CARD")]
    AskForCard {
        #[serde(rename = "targetPlayerId")]
        target_player_id: Uuid,
        rank: Rank,
    },
    #[serde(rename = "HIT")]
    Hit,
    #[serde(rename = "STAND")]
    Stand,
    #[serde(rename = "FLIP_CARD")]
    FlipCard,
}



#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoFishOpponent {
    pub player_id: Uuid,
    pub name: String,
    pub card_count: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GoFishStateView {
    pub hand: Vec<Card>,
    pub opponents: Vec<GoFishOpponent>,
    pub current_turn: Uuid,
    pub pairs: HashMap<String, u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<String>,
    pub deck_remaining: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlackjackPlayerStatus {
    Playing,
    Stood,
    Busted,
    Blackjack,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlackjackPhase {
    Dealing,
    PlayerTurns,
    DealerTurn,
    Finished,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlackjackResult {
    Win,
    Lose,
    Push,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlackjackPlayerView {
    pub player_id: Uuid,
    pub name: String,
    pub hand: Vec<Card>,
    pub score: u32,
    pub status: BlackjackPlayerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<BlackjackResult>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlackjackStateView {
    pub players: Vec<BlackjackPlayerView>,
    pub dealer_hand: Vec<Option<Card>>,
    pub dealer_score: Option<u32>,
    pub current_turn: Option<Uuid>,
    pub phase: BlackjackPhase,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarPhase {
    Flip,
    War,
    Finished,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WarPlayerView {
    pub player_id: Uuid,
    pub name: String,
    pub card_count: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WarStateView {
    pub players: Vec<WarPlayerView>,
    pub current_battle: Option<HashMap<String, Card>>,
    pub war_pile_count: usize,
    pub phase: WarPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_result: Option<String>,
}



#[derive(Serialize, Clone, Debug)]
pub struct WsEnvelope {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

impl WsEnvelope {
    pub fn new(event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            payload,
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    }
}



pub struct ActionResult {
    pub description: String,
    pub action_name: String,
    pub result_label: Option<String>,
    pub details: serde_json::Value,
    pub game_over: Option<GameOverInfo>,
}

pub struct GameOverInfo {
    pub winner_id: Option<Uuid>,
    pub winner_name: Option<String>,
    pub final_scores: HashMap<String, u32>,
    pub reason: String,
}
