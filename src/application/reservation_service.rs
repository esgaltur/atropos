use crate::domain::repository::{AllocationRepository, PlatformRepository};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

/// Background service that promotes due reservations into real leases.
///
/// It periodically scans for `PENDING` reservations whose `start_at` has passed
/// and attempts an allocation for each. On success the reservation is marked
/// `FULFILLED` with the granted lease id; on failure it is marked `FAILED` with
/// the error so the outcome is observable.
pub struct ReservationService<R: PlatformRepository + AllocationRepository> {
    repo: Arc<R>,
    batch_size: i64,
    interval_secs: u64,
}

impl<R: PlatformRepository + AllocationRepository> ReservationService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            batch_size: 100,
            interval_secs: 15,
        }
    }

    pub async fn run(&self) {
        let mut interval = time::interval(Duration::from_secs(self.interval_secs));
        loop {
            interval.tick().await;
            self.promote_due().await;
        }
    }

    /// Promotes all currently-due reservations. Returns the number processed.
    pub async fn promote_due(&self) -> i64 {
        let due = match self.repo.list_due_reservations(self.batch_size).await {
            Ok(due) => due,
            Err(e) => {
                error!(
                    "ReservationService failed to list due reservations: {:?}",
                    e
                );
                return 0;
            }
        };

        let mut processed = 0;
        for reservation in due {
            let result = self
                .repo
                .allocate_resource(
                    reservation.pool_type.clone(),
                    reservation.owner_id.clone(),
                    reservation.tenant_id.clone(),
                    reservation.priority,
                    reservation.ttl_seconds,
                    reservation.constraints.clone(),
                    None,
                    None,
                    None,
                    false,
                )
                .await;

            match result {
                Ok(lease) => {
                    if let Err(e) = self
                        .repo
                        .complete_reservation(&reservation.id, &lease.id)
                        .await
                    {
                        error!(
                            "Promoted reservation {} but failed to mark fulfilled: {:?}",
                            reservation.id, e
                        );
                    } else {
                        info!(
                            "Promoted reservation {} into lease {}",
                            reservation.id, lease.id
                        );
                    }
                }
                Err(e) => {
                    let _ = self
                        .repo
                        .fail_reservation(&reservation.id, &e.to_string())
                        .await;
                    info!(
                        "Reservation {} could not be fulfilled: {}",
                        reservation.id, e
                    );
                }
            }
            processed += 1;
        }
        processed
    }
}
