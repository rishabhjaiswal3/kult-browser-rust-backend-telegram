use kult_browser_backend_rust::external::compute::ZgComputeClient;
use kult_browser_backend_rust::external::da::ZgDaClient;
use kult_browser_backend_rust::moments::da_events::MomentDAEventRepository;
use kult_browser_backend_rust::moments::social_media::repository::post_repository::PostRepository;
use kult_browser_backend_rust::moments::social_media::worker::{PostScrapeWorker, SCRAPE_QUEUE};
use kult_browser_backend_rust::moments::{
    ComputeWorker, DAEventWorker, MigrationWorker, MomentsRepository, MIGRATION_QUEUE,
};
use kult_browser_backend_rust::onchain::{OnchainActivityRepository, OnchainActivityWorker};
use kult_browser_backend_rust::player::repository::PlayerRepository;
use kult_browser_backend_rust::redis::{connect as valkey_connect, ValkyQueue};
use kult_browser_backend_rust::referral::service::ReferralService;
use kult_browser_backend_rust::referral::worker::EvaluationWorker;
use kult_browser_backend_rust::referral::VERIFY_QUEUE;
use kult_browser_backend_rust::{logging, mongo, server};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    // Redis TLS via rustls requires an installed process-wide crypto provider.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging FIRST
    logging::init();

    // Connect to MongoDB - exit if it fails
    let db = match mongo::connect().await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(error = %e, "Failed to connect to MongoDB, shutting down");
            std::process::exit(1);
        }
    };

    // Ensure indexes exist (idempotent, logs errors but doesn't crash)
    mongo::ensure_indexes(&db).await;

    // Create a broadcast channel for graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Setup signal handler for graceful shutdown (Ctrl-C / SIGTERM)
    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        tracing::info!("Shutdown signal received. Initiating graceful shutdown...");
        let _ = shutdown_tx.send(true);
    });

    let mut worker_handles = Vec::new();

    if kult_browser_backend_rust::config::CONFIG
        .onchain
        .can_submit_transactions()
    {
        let onchain_repo = OnchainActivityRepository::new(&db);
        let onchain_worker = OnchainActivityWorker::new(onchain_repo, shutdown_rx.clone());
        let handle = tokio::spawn(async move {
            onchain_worker.run().await;
        });
        worker_handles.push(handle);
        tracing::info!("Onchain activity worker spawned as background task");
    } else {
        tracing::info!("Onchain activity worker not started");
    }

    // DA Event Worker — uploads event blobs to 0G DA (requires ZG_DA_DISPERSER_URL)
    if let Some(da_client) = ZgDaClient::from_config() {
        let da_event_repo = MomentDAEventRepository::new(&db);
        let da_worker = DAEventWorker::new(da_event_repo, da_client, shutdown_rx.clone());
        let handle = tokio::spawn(async move {
            da_worker.run().await;
        });
        worker_handles.push(handle);
        tracing::info!("DA event worker spawned as background task");
    } else {
        tracing::warn!("DA event worker not started — set ZG_DA_DISPERSER_URL to enable real 0G DA");
    }

    // 0G Compute Worker — AI analysis of stored moments (optional, requires ZG_COMPUTE_* env vars)
    if kult_browser_backend_rust::config::CONFIG.zg.has_compute() {
        if let Some(compute_client) = ZgComputeClient::from_config() {
            let compute_repo = MomentsRepository::new(&db);
            let compute_da_repo = MomentDAEventRepository::new(&db);
            let compute_worker = ComputeWorker::new(compute_repo, compute_client, shutdown_rx.clone())
                .with_da_events(compute_da_repo);
            let handle = tokio::spawn(async move {
                compute_worker.run().await;
            });
            worker_handles.push(handle);
            tracing::info!("0G Compute worker spawned as background task");
        }
    } else {
        tracing::warn!("0G Compute worker not started — set ZG_COMPUTE_PROVIDER_URL and ZG_COMPUTE_API_KEY to enable AI analysis");
    }

    // Spawn background workers
    match valkey_connect().await {
        Ok(valkey_client) => {
            // Migration worker
            match ValkyQueue::new(valkey_client.clone(), MIGRATION_QUEUE.as_str()).await {
                Ok(migration_queue) => {
                    let repo = MomentsRepository::new(&db);
                    let migration_onchain_service = if kult_browser_backend_rust::config::CONFIG
                        .onchain
                        .can_submit_transactions()
                    {
                        Some(
                            kult_browser_backend_rust::onchain::OnchainActivityService::new(
                                OnchainActivityRepository::new(&db),
                            ),
                        )
                    } else {
                        None
                    };
                    let migration_da_repo = MomentDAEventRepository::new(&db);
                    let worker = MigrationWorker::new(
                        migration_queue,
                        repo,
                        migration_onchain_service,
                        shutdown_rx.clone(),
                    )
                    .with_da_events(migration_da_repo);
                    let handle = tokio::spawn(async move { worker.run().await });
                    worker_handles.push(handle);
                    tracing::info!("Migration worker spawned as background task");
                }
                Err(e) => tracing::warn!(error = %e, "Migration worker not started — queue unavailable"),
            }

            // Post scrape worker
            match ValkyQueue::new(valkey_client.clone(), SCRAPE_QUEUE.as_str()).await {
                Ok(scrape_queue) => {
                    let post_repo = PostRepository::new(&db);
                    let scrape_worker = PostScrapeWorker::new(scrape_queue, post_repo, shutdown_rx.clone());
                    let handle = tokio::spawn(async move { scrape_worker.run().await });
                    worker_handles.push(handle);
                    tracing::info!("Post scrape worker spawned as background task");
                }
                Err(e) => tracing::warn!(error = %e, "Post scrape worker not started — queue unavailable"),
            }

            // Referral evaluation worker
            match ValkyQueue::new(valkey_client.clone(), VERIFY_QUEUE.as_str()).await {
                Ok(verify_queue) => {
                    let player_repo = Arc::new(PlayerRepository::new(&db));
                    let referral_service = Arc::new(ReferralService::new(player_repo, Some(valkey_client)));
                    let eval_worker = EvaluationWorker::new(verify_queue, referral_service, db.clone());
                    let eval_rx = shutdown_rx.clone();
                    let handle = tokio::spawn(async move { eval_worker.run(eval_rx).await });
                    worker_handles.push(handle);
                    tracing::info!("Referral evaluation worker spawned as background task");
                }
                Err(e) => tracing::warn!(error = %e, "Referral eval worker not started — queue unavailable"),
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Valkey not available — background workers disabled");
        }
    }

    // Start the server (blocks until shutdown signal is received or crashes)
    if let Err(e) = server::run(db, shutdown_rx.clone()).await {
        tracing::error!(error = %e, "Server error, shutting down");
    }

    // After server shuts down from signal, wait for background workers to finish in-flight jobs
    tracing::info!("Waiting for background workers to complete active jobs...");
    for handle in worker_handles {
        let _ = handle.await;
    }

    tracing::info!("All processes successfully shut down. Goodbye!");
}
