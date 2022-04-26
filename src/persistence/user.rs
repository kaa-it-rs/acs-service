use anyhow::Result;
use bson::oid::ObjectId;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct UserEntity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub login: String,
    pub password: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,

    #[serde(rename = "updatedAt")]
    pub updated_at: Option<bson::DateTime>,

    #[serde(rename = "roleId")]
    pub role_id: ObjectId,
}

pub(crate) async fn get_user_by_login(db: &Database, login: &String) -> Result<Option<UserEntity>> {
    let users = db.collection::<UserEntity>("users");

    let user = users
        .find_one(
            doc! {
                "login": login
            },
            None,
        )
        .await?;

    Ok(user)
}

pub(crate) async fn get_user_by_id(db: &Database, id: &String) -> Result<Option<UserEntity>> {
    let users = db.collection::<UserEntity>("users");

    let user = users
        .find_one(
            doc! {
                "_id": bson::oid::ObjectId::from_str(id)?
            },
            None,
        )
        .await?;

    Ok(user)
}

pub(crate) async fn get_users_by_id(db: &Database, ids: &[String]) -> Result<Vec<UserEntity>> {
    let users = db.collection::<UserEntity>("users");

    let ids = ids
        .into_iter()
        .map(|k| ObjectId::from_str(k).map_err(anyhow::Error::from))
        .collect::<Result<Vec<ObjectId>>>()?;

    let filter = doc! {
        "_id": {
            "$in": ids
        }
    };

    let mut cursor = users.find(filter, None).await?;

    let mut result: Vec<UserEntity> = Vec::new();
    while let Some(user) = cursor.next().await {
        result.push(user?);
    }

    Ok(result)
}
