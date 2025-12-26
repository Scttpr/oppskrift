//! Background job infrastructure
//!
//! Provides async job queue for federation delivery and other background tasks.

pub mod cleanup;
pub mod federation;

use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub use cleanup::{CleanupWorker, RetentionConfig};
pub use federation::{FederationJob, FederationWorker};

/// Job queue for background processing
#[derive(Clone)]
pub struct JobQueue {
    federation_tx: mpsc::Sender<FederationJob>,
}

impl JobQueue {
    /// Create a new job queue and spawn worker tasks
    pub fn new(pool: Arc<PgPool>) -> Self {
        let (federation_tx, federation_rx) = mpsc::channel(100);

        // Spawn federation worker
        let worker = FederationWorker::new(pool);
        tokio::spawn(worker.run(federation_rx));

        Self { federation_tx }
    }

    /// Queue a federation delivery job
    pub async fn queue_federation(
        &self,
        job: FederationJob,
    ) -> Result<(), mpsc::error::SendError<FederationJob>> {
        self.federation_tx.send(job).await
    }
}
