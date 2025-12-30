use base64::{engine::general_purpose, Engine};
use linera_base::identifiers::AccountOwner;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SwissPlayer {
    player_id: AccountOwner,
    score: u8, // starts at 0
    opponents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SingleElimPlayer {
    player_id: AccountOwner,
    score: u8, // starts at 0
    opponents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SwissParticipants {
    pub players: Vec<SwissPlayer>,
    pub max_players: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SingleElimParticipants {
    pub players: Vec<SingleElimPlayer>,
    pub max_players: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Participants {
    Swiss(SwissParticipants),
    SingleElim(SingleElimParticipants),
}

impl Participants {
    pub fn decode(encoded: String) -> Self {
        let bytes = general_purpose::STANDARD
            .decode(encoded)
            .expect("invalid base64 input");

        postcard::from_bytes::<Participants>(&bytes).expect("postcard deserialization failed")
    }
}

pub trait TournamentParticipants: std::fmt::Debug {}

impl TournamentParticipants for SwissPlayer {}

impl TournamentParticipants for SingleElimPlayer {}
