//! PostgreSQL implementation of repository traits.

use sqlx::PgPool;

/// Maximum retries on serialization failure (Postgres error 40001).
pub(crate) const SERIALIZABLE_MAX_RETRIES: u32 = 3;

/// Check whether a sqlx error is a Postgres serialization failure (40001).
pub(crate) fn is_serialization_failure(e: &sqlx::Error) -> bool {
    matches!(
        e,
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("40001")
    )
}

/// Check whether an anyhow error wraps a serialization failure.
#[allow(dead_code)]
pub(crate) fn is_serialization_failure_anyhow(e: &anyhow::Error) -> bool {
    e.downcast_ref::<sqlx::Error>()
        .is_some_and(is_serialization_failure)
}

/// Run a block inside a SERIALIZABLE transaction with automatic retry on
/// serialization failure (Postgres error code 40001). The block receives `$tx`
/// as a mutable transaction — use `&mut *$tx` for query execution. The macro
/// handles commit; do NOT commit inside the block. Use `?` normally — errors
/// are caught and classified (serialization failures trigger retry, others
/// propagate immediately).
macro_rules! run_serializable {
    ($pool:expr, |$tx:ident| { $($body:tt)* }) => {{
        let mut _last_err: Option<anyhow::Error> = None;
        let mut _succeeded = false;
        for _attempt in 0..$crate::db::pg_repo::SERIALIZABLE_MAX_RETRIES {
            let mut $tx = $pool.begin().await?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                .execute(&mut *$tx)
                .await?;

            let _body_result: ::anyhow::Result<()> = (async {
                $($body)*
            }).await;

            match _body_result {
                Ok(()) => match $tx.commit().await {
                    Ok(()) => {
                        _succeeded = true;
                        break;
                    }
                    Err(e) if $crate::db::pg_repo::is_serialization_failure(&e) => {
                        tracing::warn!(
                            attempt = _attempt,
                            "serialization failure on commit, retrying"
                        );
                        _last_err = Some(anyhow::Error::from(e));
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                },
                Err(e) if $crate::db::pg_repo::is_serialization_failure_anyhow(&e) => {
                    tracing::warn!(attempt = _attempt, "serialization failure, retrying");
                    _last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        if !_succeeded {
            return Err(_last_err.unwrap_or_else(|| {
                anyhow::anyhow!("serializable transaction failed after max retries")
            }));
        }
        Ok(())
    }};
}

/// Production repository backed by PostgreSQL.
#[derive(Clone)]
pub struct PgRepo {
    pool: PgPool,
}

impl PgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

mod agent;
mod auth;
mod collection;
mod content_version;
mod cost;
mod document;
mod execution;
mod protocol;
mod room;
mod session;
mod system_config;
mod tool;
mod tool_capability;
mod user;
mod workflow;

#[cfg(test)]
mod tests;
