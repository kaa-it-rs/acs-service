use anyhow::Result;
use bson::doc;
use chrono::{Duration, Local};
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientEntity {
    pub user_id: bson::oid::ObjectId,
    pub refresh_token: String,
    pub expired: i64,
}

const REFRESH_TOKEN_DURATION_IN_MINUTES: i64 = 1440;

pub(crate) async fn create_client(
    db: &Database,
    refresh_token: String,
    user_id: &str,
) -> Result<()> {
    let clients = db.collection::<ClientEntity>("clients");

    let exp_time = Local::now() + Duration::minutes(REFRESH_TOKEN_DURATION_IN_MINUTES);
    let user_oid = bson::oid::ObjectId::from_str(user_id)?;

    let client = ClientEntity {
        refresh_token,
        user_id: user_oid,
        expired: exp_time.timestamp(),
    };

    clients.insert_one(client, None).await?;

    Ok(())
}

pub(crate) async fn remove_expired_clients(db: &Database) -> Result<()> {
    let clients = db.collection::<ClientEntity>("clients");

    let now = Local::now();

    let filter = doc! {
        "expired": {
            "$lt": now.timestamp()
        }
    };

    clients.delete_many(filter, None).await?;

    Ok(())
}

pub(crate) async fn client(db: &Database, refresh_token: String) -> Result<Option<ClientEntity>> {
    let clients = db.collection::<ClientEntity>("clients");

    let filter = doc! {
        "refreshToken": refresh_token
    };

    let client = clients.find_one(filter, None).await?;

    Ok(client)
}

pub(crate) async fn update_client(
    db: &Database,
    refresh_token: String,
    new_refresh_token: String,
) -> Result<()> {
    let clients = db.collection::<ClientEntity>("clients");

    let exp_time = Local::now() + Duration::minutes(40);

    let filter = doc! {
        "refreshToken": refresh_token
    };

    let update = doc! {
        "$set": {
            "refreshToken": new_refresh_token,
            "expired": exp_time.timestamp(),
        }
    };

    clients.update_one(filter, update, None).await?;

    Ok(())
}
