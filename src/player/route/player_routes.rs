use axum::{
    routing::{get, patch, post},
    Router,
};
use mongodb::{Client, Database};

use crate::leaderboard::repository::{
    GameLeaderboardConfigRepository, GlobalLeaderboardRepository,
};
use crate::leaderboard::service::GameLeaderboardService;
use crate::player::controller::{
    get_nonce, get_profile, login, privy_login, update_name, PlayerState,
};
use crate::player::repository::PlayerRepository;
use crate::player::service::PlayerService;
use crate::player::siwe::NonceRepository;

use crate::agent::repository::agent_repository::AgentRepository;

/// Build routes for the player module.
///
/// Routes:
/// - GET /nonce  - Issue a SIWE nonce for the given wallet
/// - POST /login - Login or register (SIWE-verified)
/// - GET /profile - Get player profile (auth required)
/// - PATCH /name - Update player name (auth required)
pub fn routes(
    db: Database,
    client: Client,
    anti_fraud_service: Option<std::sync::Arc<crate::referral::anti_fraud::AntiFraudService>>,
) -> Router {
    let player_repo = PlayerRepository::new(&db);
    let global_lb_repo = GlobalLeaderboardRepository::new(&db);
    let config_repo = GameLeaderboardConfigRepository::new(&db);
    let agent_repo = AgentRepository::new(&db);
    let game_lb_service = GameLeaderboardService::new(config_repo, client);
    let nonce_repo = NonceRepository::new(&db);

    let player_service = PlayerService::new(
        player_repo,
        global_lb_repo,
        game_lb_service,
        agent_repo,
        anti_fraud_service,
        nonce_repo,
    );
    let state = PlayerState { player_service };

    Router::new()
        .route("/nonce", get(get_nonce))
        .route("/login", post(login))
        .route("/privy-login", post(privy_login))
        .route("/profile", get(get_profile))
        .route("/name", patch(update_name))
        .with_state(state)
}
