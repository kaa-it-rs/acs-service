use anyhow::Result;
use bson::oid::ObjectId;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BaseRights {
    pub list: bool,
    pub view: bool,
    pub create: bool,
    pub edit: bool,
    pub delete: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AccessRights {
    pub users: BaseRights,
    pub roles: BaseRights,
    pub openers: BaseRights,
    #[serde(rename = "barrierManufacturers")]
    pub barrier_manufacturers: BaseRights,
    #[serde(rename = "barrierModels")]
    pub barrier_models: BaseRights,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RoleEntity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    #[serde(rename = "accessRights")]
    pub access_rights: AccessRights,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
}

pub(crate) async fn get_role_by_id(db: &Database, id: &str) -> Result<Option<RoleEntity>> {
    let roles = db.collection::<RoleEntity>("roles");

    let role = roles
        .find_one(
            doc! {
                "_id": ObjectId::from_str(id)?
            },
            None,
        )
        .await?;

    Ok(role)
}

pub(crate) async fn get_roles_by_id(db: &Database, ids: &[String]) -> Result<Vec<RoleEntity>> {
    let roles = db.collection::<RoleEntity>("roles");

    let ids = ids
        .iter()
        .map(|k| ObjectId::from_str(k).map_err(anyhow::Error::from))
        .collect::<Result<Vec<ObjectId>>>()?;

    let filter = doc! {
        "_id": {
            "$in": ids
        }
    };

    let mut cursor = roles.find(filter, None).await?;

    let mut result: Vec<RoleEntity> = Vec::new();
    while let Some(role) = cursor.next().await {
        result.push(role?);
    }

    Ok(result)
}
