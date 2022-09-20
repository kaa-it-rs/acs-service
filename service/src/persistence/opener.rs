use crate::persistence::utils::check_already_exists;
use anyhow::Result;
use bson::oid::ObjectId;
use bson::Document;
use chrono::Local;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct NewOpenerEntity {
    #[serde(rename = "serialNumber")]
    pub serial_number: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct UpdateOpenerEntity {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub login: Option<String>,
    pub password: Option<String>,
    pub nonce: Option<String>,
    pub version: Option<String>,
    pub connected: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct OpenerEntity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "serialNumber")]
    pub serial_number: String,
    pub version: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub login: String,
    pub password: String,
    pub nonce: Option<String>,
    pub connected: bool,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<bson::DateTime>,

    #[serde(rename = "barrierModelId")]
    pub barrier_model_id: Option<ObjectId>,

    #[serde(rename = "userId")]
    pub user_id: Option<ObjectId>,
}

pub(crate) async fn create_opener(
    db: &Database,
    new_opener: &NewOpenerEntity,
) -> Result<OpenerEntity> {
    let docs = db.collection::<Document>("openers");

    let opener = doc! {
        "serialNumber": new_opener.serial_number.clone(),
        "connected": false,
        "login": "admin",
        "password": "admin",
        "createdAt": bson::DateTime::from(Local::now())
    };

    let result = match docs.insert_one(opener, None).await {
        Err(e) => return Err(check_already_exists(e)),
        Ok(result) => result,
    };

    let openers = db.collection::<OpenerEntity>("openers");

    Ok(openers
        .find_one(doc! { "_id": result.inserted_id }, None)
        .await?
        .unwrap())
}

pub(crate) async fn update_opener(
    db: &Database,
    serial_number: &String,
    new_opener: &UpdateOpenerEntity,
) -> Result<OpenerEntity> {
    let docs = db.collection::<Document>("openers");

    let mut opener = doc! {
        "updatedAt": bson::DateTime::from(Local::now())
    };

    if new_opener.user_id.is_some() {
        opener.insert(
            "userId",
            ObjectId::from_str(new_opener.user_id.as_ref().unwrap())?,
        );
    }

    if new_opener.alias.is_some() {
        opener.insert("alias", new_opener.alias.clone());
    }

    if new_opener.description.is_some() {
        opener.insert("description", new_opener.description.clone());
    }

    if new_opener.lat.is_some() {
        opener.insert("lat", new_opener.lat);
    }

    if new_opener.lng.is_some() {
        opener.insert("lng", new_opener.lng);
    }

    if new_opener.login.is_some() {
        opener.insert("login", new_opener.login.clone());
    }

    if new_opener.password.is_some() {
        opener.insert("password", new_opener.password.clone());
    }

    if new_opener.version.is_some() {
        opener.insert("version", new_opener.version.clone());
    }

    if new_opener.nonce.is_some() {
        opener.insert("nonce", new_opener.nonce.clone());
    }

    if new_opener.connected.is_some() {
        opener.insert("connected", new_opener.connected);
    }

    let filter = doc! {
        "serialNumber": serial_number
    };

    let update = doc! {
        "$set": opener
    };

    docs.update_one(filter.clone(), update, None).await?;

    let openers = db.collection::<OpenerEntity>("openers");

    Ok(openers.find_one(filter, None).await?.unwrap())
}

pub(crate) async fn get_opener_by_id(db: &Database, id: &str) -> Result<Option<OpenerEntity>> {
    let openers = db.collection::<OpenerEntity>("openers");

    let opener = openers
        .find_one(
            doc! {
                "_id": ObjectId::from_str(id)?
            },
            None,
        )
        .await?;

    Ok(opener)
}

pub(crate) async fn get_opener_by_sn(
    db: &Database,
    serial_number: &String,
) -> Result<Option<OpenerEntity>> {
    let openers = db.collection::<OpenerEntity>("openers");

    let opener = openers
        .find_one(
            doc! {
                "serialNumber": serial_number
            },
            None,
        )
        .await?;

    Ok(opener)
}

pub(crate) async fn get_openers(
    db: &Database,
    user_id: Option<&String>,
) -> Result<Vec<OpenerEntity>> {
    log::info!("Get openers");
    let openers = db.collection::<OpenerEntity>("openers");

    let mut filter = bson::Document::new();

    if user_id.is_some() {
        filter.insert("userId", ObjectId::from_str(user_id.unwrap())?);
    }

    let mut cursor = openers.find(filter, None).await?;

    let mut openers: Vec<OpenerEntity> = Vec::new();
    while let Some(opener) = cursor.next().await {
        openers.push(opener?);
    }

    Ok(openers)
}
