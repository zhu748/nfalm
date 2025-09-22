use std::sync::LazyLock;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Schema};

use crate::error::ClewdrError;

use super::entities::{
    ColumnCookie, ColumnKeyRow, EntityConfig, EntityCookie, EntityKeyRow, EntityWasted,
};

static CONN: LazyLock<std::sync::Mutex<Option<DatabaseConnection>>> =
    LazyLock::new(|| std::sync::Mutex::new(None));

pub async fn ensure_conn() -> Result<DatabaseConnection, ClewdrError> {
    if let Ok(Some(db)) = CONN.lock().map(|g| g.as_ref().cloned()) {
        return Ok(db);
    }
    let cfg = crate::config::CLEWDR_CONFIG.load();
    if !cfg.is_db_mode() {
        return Err(ClewdrError::Whatever {
            message: "DB mode not enabled".into(),
            source: None,
        });
    }
    let url = cfg.database_url().ok_or(ClewdrError::UnexpectedNone {
        msg: "Database URL not provided",
    })?;
    if url.starts_with("sqlite://") && !cfg.no_fs {
        if let Some(parent) = std::path::Path::new(&url["sqlite://".len()..]).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let db = Database::connect(&url)
        .await
        .map_err(|e| ClewdrError::Whatever {
            message: "db_connect".into(),
            source: Some(Box::new(e)),
        })?;
    migrate(&db).await?;
    if let Ok(mut g) = CONN.lock() {
        *g = Some(db.clone());
    }
    Ok(db)
}

async fn migrate(db: &DatabaseConnection) -> Result<(), ClewdrError> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let stmt: sea_orm::sea_query::TableCreateStatement =
        schema.create_table_from_entity(EntityConfig);
    db.execute(backend.build(&stmt)).await.ok();
    let stmt = schema.create_table_from_entity(EntityCookie);
    db.execute(backend.build(&stmt)).await.ok();
    let stmt = schema.create_table_from_entity(EntityWasted);
    db.execute(backend.build(&stmt)).await.ok();
    let stmt = schema.create_table_from_entity(EntityKeyRow);
    db.execute(backend.build(&stmt)).await.ok();
    // indexes
    use sea_orm::sea_query::Index;
    // cookies(token_org_uuid)
    let idx = Index::create()
        .name("idx_cookies_org_uuid")
        .table(EntityCookie)
        .col(ColumnCookie::TokenOrgUuid)
        .to_owned();
    db.execute(backend.build(&idx)).await.ok();
    // cookies(reset_time)
    let idx = Index::create()
        .name("idx_cookies_reset")
        .table(EntityCookie)
        .col(ColumnCookie::ResetTime)
        .to_owned();
    db.execute(backend.build(&idx)).await.ok();
    // keys(count_403)
    let idx = Index::create()
        .name("idx_keys_count")
        .table(EntityKeyRow)
        .col(ColumnKeyRow::Count403)
        .to_owned();
    db.execute(backend.build(&idx)).await.ok();
    Ok(())
}
