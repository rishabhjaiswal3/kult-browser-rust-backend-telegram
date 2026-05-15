use serde::Serialize;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::Components;
use utoipa::{Modify, OpenApi, ToSchema};

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
    pub ts: String,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct ContentApiResponse {
    pub ok: bool,
    pub data: crate::content::model::ContentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct AllGamesApiResponse {
    pub ok: bool,
    pub data: crate::game::dto::AllGamesResponse,
}

#[derive(Serialize, ToSchema)]
pub struct CategoriesApiResponse {
    pub ok: bool,
    pub data: crate::game::dto::CategoriesResponse,
}

#[derive(Serialize, ToSchema)]
pub struct GameDetailApiResponse {
    pub ok: bool,
    pub data: crate::game::dto::GameDetailResponse,
}

#[derive(Serialize, ToSchema)]
pub struct GlobalLeaderboardApiResponse {
    pub ok: bool,
    pub data: crate::leaderboard::dto::GlobalLeaderboardResponse,
}

#[derive(Serialize, ToSchema)]
pub struct GameLeaderboardApiResponse {
    pub ok: bool,
    pub data: crate::leaderboard::dto::GameLeaderboardResponse,
}

#[derive(Serialize, ToSchema)]
pub struct RefreshLeaderboardResponse {
    pub refreshed: usize,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct RefreshLeaderboardApiResponse {
    pub ok: bool,
    pub data: RefreshLeaderboardResponse,
}

#[derive(Serialize, ToSchema)]
pub struct NonceApiResponse {
    pub ok: bool,
    pub data: crate::player::dto::NonceResponse,
}

#[derive(Serialize, ToSchema)]
pub struct LoginApiResponse {
    pub ok: bool,
    pub data: crate::player::dto::LoginResponse,
}

#[derive(Serialize, ToSchema)]
pub struct PlayerProfileApiResponse {
    pub ok: bool,
    pub data: crate::player::dto::PlayerProfileResponse,
}

#[derive(Serialize, ToSchema)]
pub struct UpdateNameApiResponse {
    pub ok: bool,
    pub data: crate::player::dto::UpdateNameResponse,
}

#[derive(Serialize, ToSchema)]
pub struct CreateMomentApiResponse {
    pub ok: bool,
    pub data: crate::moments::dto::CreateMomentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct MomentListApiResponse {
    pub ok: bool,
    pub data: crate::moments::dto::MomentListResponse,
}

#[derive(Serialize, ToSchema)]
pub struct MomentApiResponse {
    pub ok: bool,
    pub data: crate::moments::dto::MomentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct MomentZgProofApiResponse {
    pub ok: bool,
    pub data: crate::moments::dto::MomentZgProofResponse,
}

#[derive(Serialize, ToSchema)]
pub struct RetryZgMigrationApiResponse {
    pub ok: bool,
    pub data: crate::moments::dto::RetryZgMigrationResponse,
}

#[derive(Serialize, ToSchema)]
pub struct CommentApiResponse {
    pub ok: bool,
    pub data: crate::moments::comments::dto::CommentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct CommentListApiResponse {
    pub ok: bool,
    pub data: crate::moments::comments::dto::CommentListResponse,
}

#[derive(Serialize, ToSchema)]
pub struct CreatorProfileApiResponse {
    pub ok: bool,
    pub data: crate::moments::creators::dto::CreatorProfileResponse,
}

#[derive(Serialize, ToSchema)]
pub struct CreatorLeaderboardApiResponse {
    pub ok: bool,
    pub data: crate::moments::creators::dto::CreatorLeaderboardResponse,
}

#[derive(Serialize, ToSchema)]
pub struct LikeMomentApiResponse {
    pub ok: bool,
    pub data: crate::moments::dto::LikeMomentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteMomentResponse {
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteMomentApiResponse {
    pub ok: bool,
    pub data: DeleteMomentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteCommentApiResponse {
    pub ok: bool,
    pub data: crate::moments::comments::dto::DeleteCommentResponse,
}

#[derive(Serialize, ToSchema)]
pub struct SubmitSharedPostResponse {
    #[serde(rename = "postId")]
    pub post_id: String,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct SubmitSharedPostApiResponse {
    pub ok: bool,
    pub data: SubmitSharedPostResponse,
}

#[derive(Serialize, ToSchema)]
pub struct SharedPostListApiResponse {
    pub ok: bool,
    pub data: crate::moments::social_media::dto::SharedPostListResponse,
}

#[derive(Serialize, ToSchema)]
pub struct RequeueSharedPostApiResponse {
    pub ok: bool,
    pub data: crate::moments::social_media::dto::RequeueSharedPostResponse,
}

#[derive(Serialize, ToSchema)]
pub struct PresignApiResponse {
    pub ok: bool,
    pub data: crate::upload::controller::PresignResponse,
}

#[derive(Serialize, ToSchema)]
pub struct ReferralLinkResponse {
    pub code: String,
    pub link: String,
}

#[derive(Serialize, ToSchema)]
pub struct ReferralErrorResponse {
    pub error: String,
}

#[derive(Serialize, ToSchema)]
pub struct ListingApiResponse {
    pub ok: bool,
    pub data: crate::marketplace::dto::ListingResponse,
}

#[derive(Serialize, ToSchema)]
pub struct ListingListApiResponse {
    pub ok: bool,
    pub data: crate::marketplace::dto::ListingListResponse,
}

#[derive(Serialize, ToSchema)]
pub struct OrderApiResponse {
    pub ok: bool,
    pub data: crate::marketplace::orders::dto::OrderResponse,
}

#[derive(Serialize, ToSchema)]
pub struct OrderListApiResponse {
    pub ok: bool,
    pub data: crate::marketplace::orders::dto::OrderListResponse,
}

#[derive(Serialize, ToSchema)]
pub struct PrepareOrderApiResponse {
    pub ok: bool,
    pub data: crate::marketplace::orders::dto::PrepareOrderResponse,
}

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::server::health_check,
        crate::content::controller::content_controller::get_content,
        crate::game::controller::game_controller::get_all_games,
        crate::game::controller::game_controller::get_all_categories,
        crate::game::controller::game_controller::get_game_by_id,
        crate::leaderboard::controller::leaderboard_controller::get_global_leaderboard,
        crate::leaderboard::controller::game_leaderboard_controller::get_game_leaderboard,
        crate::leaderboard::controller::leaderboard_controller::refresh_global_leaderboard,
        crate::player::controller::player_controller::login,
        crate::player::controller::player_controller::get_profile,
        crate::player::controller::player_controller::update_name,
        crate::moments::controller::moments_controller::create_moment,
        crate::moments::controller::moments_controller::get_feed,
        crate::moments::controller::moments_controller::get_my_moments,
        crate::moments::controller::moments_controller::get_moment,
        crate::moments::controller::moments_controller::get_zg_proof,
        crate::moments::controller::moments_controller::retry_zg_migration,
        crate::moments::controller::moments_controller::like_moment,
        crate::moments::controller::moments_controller::update_moment,
        crate::moments::controller::moments_controller::delete_moment,
        crate::moments::creators::controller::get_me,
        crate::moments::creators::controller::get_leaderboard,
        crate::moments::comments::controller::create_comment,
        crate::moments::comments::controller::list_comments,
        crate::moments::comments::controller::create_reply,
        crate::moments::comments::controller::list_replies,
        crate::moments::comments::controller::update_comment,
        crate::moments::comments::controller::delete_comment,
        crate::moments::social_media::controller::submit_post,
        crate::moments::social_media::controller::list_my_posts,
        crate::moments::social_media::controller::requeue_post,
        crate::upload::controller::generate_presigned_url,
        crate::referral::route::get_my_referral_code,
        crate::referral::redirect_route::handle_redirect,
        crate::marketplace::controller::listing_controller::get_listings,
        crate::marketplace::controller::listing_controller::get_listing,
        crate::marketplace::orders::controller::order_controller::create_order,
        crate::marketplace::orders::controller::order_controller::prepare_order,
        crate::marketplace::orders::controller::order_controller::confirm_order,
        crate::marketplace::orders::controller::order_controller::get_orders,
        crate::marketplace::orders::controller::order_controller::get_order
    ),
    components(
        schemas(
            crate::content::model::ContentResponse,
            crate::content::model::ContentItem,
            crate::content::model::MappedContentItem,
            crate::game::dto::AllGamesResponse,
            crate::game::dto::CategoriesResponse,
            crate::game::dto::GameDetailDto,
            crate::game::dto::GameDetailResponse,
            crate::game::dto::GameListItemDto,
            crate::game::model::GameModel,
            crate::game::model::GameImages,
            crate::game::model::ImageObject,
            crate::game::model::IndexedImage,
            crate::game::model::Localized<crate::game::model::ImageObject>,
            crate::game::model::Localized<String>,
            crate::game::model::OrientedImage,
            crate::game::model::OrientedImageArray,
            crate::leaderboard::dto::GameLeaderboardEntryDto,
            crate::leaderboard::dto::GameLeaderboardResponse,
            crate::leaderboard::dto::GlobalLeaderboardEntryDto,
            crate::leaderboard::dto::GlobalLeaderboardResponse,
            crate::moments::comments::dto::CommentListResponse,
            crate::moments::comments::dto::CommentResponse,
            crate::moments::comments::dto::CreateCommentRequest,
            crate::moments::comments::dto::DeleteCommentResponse,
            crate::moments::comments::dto::UpdateCommentRequest,
            crate::moments::creators::dto::CreatorLeaderboardEntry,
            crate::moments::creators::dto::CreatorLeaderboardResponse,
            crate::moments::creators::dto::CreatorProfileResponse,
            crate::moments::dto::CreateMomentRequest,
            crate::moments::dto::CreateMomentResponse,
            crate::moments::dto::LikeMomentResponse,
            crate::moments::dto::MomentListResponse,
            crate::moments::dto::MomentResponse,
            crate::moments::dto::MomentZgProofResponse,
            crate::moments::dto::RetryZgMigrationResponse,
            crate::moments::dto::UpdateMomentRequest,
            crate::moments::social_media::dto::RequeueSharedPostResponse,
            crate::moments::social_media::dto::SharedPostListResponse,
            crate::moments::social_media::dto::SharedPostResponse,
            crate::moments::social_media::dto::SubmitPostRequest,
            crate::moments::social_media::model::platform::Platform,
            crate::moments::social_media::model::post_model::ValidationStatus,
            crate::player::dto::GameScoreEntry,
            crate::player::dto::LoginRequest,
            crate::player::dto::PrivyLoginRequest,
            crate::player::dto::LoginResponse,
            crate::player::dto::PlayerInfo,
            crate::player::dto::PlayerProfile,
            crate::player::dto::PlayerProfileResponse,
            crate::player::dto::UpdateNameRequest,
            crate::player::dto::UpdateNameResponse,
            crate::upload::controller::PresignRequest,
            crate::upload::controller::PresignResponse,
            AllGamesApiResponse,
            CategoriesApiResponse,
            CommentApiResponse,
            CommentListApiResponse,
            ContentApiResponse,
            CreatorLeaderboardApiResponse,
            CreatorProfileApiResponse,
            CreateMomentApiResponse,
            DeleteCommentApiResponse,
            DeleteMomentApiResponse,
            DeleteMomentResponse,
            ErrorResponse,
            GameDetailApiResponse,
            GameLeaderboardApiResponse,
            GlobalLeaderboardApiResponse,
            HealthResponse,
            LikeMomentApiResponse,
            LoginApiResponse,
            NonceApiResponse,
            MomentApiResponse,
            MomentListApiResponse,
            MomentZgProofApiResponse,
            PlayerProfileApiResponse,
            PresignApiResponse,
            RequeueSharedPostApiResponse,
            RefreshLeaderboardApiResponse,
            RefreshLeaderboardResponse,
            RetryZgMigrationApiResponse,
            ReferralErrorResponse,
            ReferralLinkResponse,
            SharedPostListApiResponse,
            SubmitSharedPostApiResponse,
            SubmitSharedPostResponse,
            UpdateNameApiResponse,
            ListingApiResponse,
            ListingListApiResponse,
            OrderApiResponse,
            OrderListApiResponse,
            PrepareOrderApiResponse,
            crate::marketplace::dto::ListingResponse,
            crate::marketplace::dto::ListingListResponse,
            crate::marketplace::dto::CreateListingRequest,
            crate::marketplace::dto::UpdateListingRequest,
            crate::marketplace::orders::dto::CreateOrderRequest,
            crate::marketplace::orders::dto::PrepareOrderRequest,
            crate::marketplace::orders::dto::ConfirmOrderRequest,
            crate::marketplace::orders::dto::PrepareOrderResponse,
            crate::marketplace::orders::dto::OrderResponse,
            crate::marketplace::orders::dto::OrderListResponse
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Service health and liveness"),
        (name = "Content", description = "Homepage and section content"),
        (name = "Games", description = "Game catalog endpoints"),
        (name = "Leaderboard", description = "Global and per-game rankings"),
        (name = "Player", description = "Player auth and profile"),
        (name = "Moments", description = "Moments feed and player moments"),
        (name = "Moments Creators", description = "Moments creator stats and leaderboard"),
        (name = "Comments", description = "Moment comments and replies"),
        (name = "Social Media", description = "Moment social media submission"),
        (name = "Upload", description = "Asset upload preparation"),
        (name = "Referral", description = "Referral management and redirects"),
        (name = "Marketplace", description = "Marketplace listings"),
        (name = "Marketplace Orders", description = "Marketplace order management")
    )
)]
pub struct ApiDoc;
