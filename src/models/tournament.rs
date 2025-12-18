use crate::supabase::{SupabaseClient, SupabaseModel};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

impl Tournament {
    pub fn for_db(&self) -> TournamentDB {
        TournamentDB {
            tournament_id: self.tournament_id.clone(),
            organiser_chain: self.organiser_chain.clone(),
            organiser_id: self.organiser_id.clone(),
            organiser_name: self.organiser_name.clone(),

            tournament_name: self.tournament_name.clone(),
            tournament_description: self.tournament_description.clone(),

            tournament_format: self.tournament_format.clone(),
            match_type: self.match_type.clone(),
            game_mode: self.game_mode.clone(),

            // Flattened TimeControl
            time_control_base_minutes: self
                .time_control
                .as_ref()
                .map(|tc| tc.base_minutes)
                .unwrap_or(0),
            time_control_increment_seconds: self
                .time_control
                .as_ref()
                .map(|tc| tc.increment_seconds)
                .unwrap_or(0),
            time_control_mode_label: self
                .time_control
                .as_ref()
                .map(|tc| tc.mode_label.clone())
                .unwrap_or(None),

            max_players: self.max_players,
            min_players: self.min_players,

            starting_time: self.starting_time,
            end_time: self.end_time,

            prize_pool_description: self.prize_pool_description.clone(),

            visibility: self.visibility.clone(),

            banner_image_url: self.banner_image_url.clone(),
            sponsor_logo_url: self.sponsor_logo_url.clone(),

            // JSONB arrays
            prize_type: self.prize_type.clone(),
            prize_pool: self.prize_pool,
            custom_tags: self.custom_tags.clone(),

            version: self.version.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            status: self.status.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentDB {
    #[serde(rename = "tournament_id")]
    pub tournament_id: String,
    pub organiser_chain: String,
    pub organiser_id: String,
    pub organiser_name: String,

    pub tournament_name: String,
    pub tournament_description: Option<String>,

    pub tournament_format: String,
    pub match_type: String,
    pub game_mode: String,

    pub time_control_base_minutes: u32,
    pub time_control_increment_seconds: u32,
    pub time_control_mode_label: Option<String>,

    pub max_players: Option<u32>,
    pub min_players: Option<u32>,

    pub starting_time: usize,
    pub end_time: usize,

    pub prize_pool_description: Option<String>,

    pub visibility: String,

    pub banner_image_url: Option<String>,
    pub sponsor_logo_url: Option<String>,

    pub prize_type: Option<String>,
    pub prize_pool: u32,
    pub custom_tags: Vec<String>,

    pub version: String,
    pub created_at: usize,
    pub updated_at: usize,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeControl {
    pub base_minutes: u32,
    pub increment_seconds: u32,
    pub mode_label: Option<String>, // optional human readable like "3+2"
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tournament {
    // --- Identity ---
    pub organiser_chain: String,
    pub organiser_id: String,
    pub organiser_name: String,
    pub tournament_id: String,
    pub tournament_name: String,
    pub tournament_description: Option<String>,

    // --- Format & Rules ---
    pub tournament_format: String,
    pub match_type: String,
    pub game_mode: String,
    pub time_control: Option<TimeControl>,
    pub max_players: Option<u32>,
    pub min_players: Option<u32>,

    // --- Schedule ---
    pub starting_time: usize,
    pub end_time: usize,

    // --- Rewards ---
    pub prize_type: Option<String>,
    pub prize_pool_description: Option<String>,
    pub prize_pool: u32,

    // --- Access & Privacy ---
    pub visibility: String,

    // --- Branding ---
    pub banner_image_url: Option<String>,
    pub sponsor_logo_url: Option<String>,
    pub custom_tags: Vec<String>,

    // --- System Metadata ---
    pub version: String,
    pub created_at: usize,
    pub updated_at: usize,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct TournamentResponse {
    pub data: Tournaments,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tournaments {
    pub all_tournaments: Vec<Tournament>,
}

pub const QUERY_TOURNAMENTS: &str = r#"{ "query": "query { allTournaments { organiserChain organiserId organiserName tournamentId tournamentName tournamentFormat matchType gameMode timeControl { baseMinutes incrementSeconds modeLabel } bannerImageUrl sponsorLogoUrl maxPlayers minPlayers startingTime endTime prizeType prizePoolDescription prizePool visibility customTags version createdAt updatedAt status } }" }"#;

pub fn participants_query(tournament_id: &str) -> String {
    format!(
        r#"{{"query": "query {{ participants(tournamentId: \"{}\") {{ id player {{ name elo matches ath }} }} }}"}}"#,
        tournament_id
    )
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlayerInfo {
    pub name: Option<String>,
    pub elo: u32,
    pub matches: u32,
    pub ath: u32,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct TournamentParticipant {
    pub id: String,
    pub player: PlayerInfo,
}

#[derive(Debug, Deserialize)]
pub struct ParticipantResponse {
    pub data: Participants,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participants {
    pub participants: Vec<TournamentParticipant>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentParticipantDB {
    pub id: String,
    pub tournament_id: String,
    pub player_name: Option<String>,
    pub player_elo: u32,
    pub player_matches: u32,
    pub player_ath: u32,
}

impl TournamentParticipant {
    pub fn for_db(&self, tournament_id: String) -> TournamentParticipantDB {
        TournamentParticipantDB {
            id: self.id.clone(),
            tournament_id,
            player_name: self.player.name.clone(),
            player_elo: self.player.elo,
            player_matches: self.player.matches,
            player_ath: self.player.ath,
        }
    }
}

#[async_trait]
impl SupabaseModel for TournamentDB {
    fn table_name() -> &'static str {
        "tournaments"
    }

    fn primary_key() -> &'static str {
        "tournament_id"
    }

    async fn insert(&self, client: &SupabaseClient) -> Result<()> {
        client.upsert(self).await
    }

    async fn insert_many(records: Vec<Self>, client: &SupabaseClient) -> Result<()> {
        client.insert_many(&records).await
    }

    async fn replace(&self, client: &SupabaseClient) -> Result<()> {
        client
            .delete_one::<Self>(&self.tournament_id)
            .await?
            .insert(self)
            .await
    }

    async fn replace_all(_records: Vec<Self>, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("replace_all not supported for tournaments")
    }
}

#[async_trait]
impl SupabaseModel for TournamentParticipantDB {
    fn table_name() -> &'static str {
        "tournament_participants"
    }

    fn primary_key() -> &'static str {
        "id"
    }

    async fn insert(&self, client: &SupabaseClient) -> Result<()> {
        client.upsert(self).await
    }

    async fn insert_many(records: Vec<Self>, client: &SupabaseClient) -> Result<()> {
        client.insert_many(&records).await
    }

    async fn replace(&self, client: &SupabaseClient) -> Result<()> {
        client
            .delete_one::<Self>(&self.tournament_id)
            .await?
            .insert(self)
            .await
    }

    async fn replace_all(_records: Vec<Self>, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("replace_all not supported for participants")
    }
}
