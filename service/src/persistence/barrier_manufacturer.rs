use anyhow::Result;
use bson::oid::ObjectId;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BarrierManufacturerEntity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<bson::DateTime>,

    #[serde(rename = "modelIds")]
    pub model_ids: Option<Vec<ObjectId>>,
}

pub(crate) async fn get_barrier_manufacturers(
    db: &Database,
) -> Result<Vec<BarrierManufacturerEntity>> {
    log::info!("Get barrier manufacturers");

    let manufacturers = db.collection::<BarrierManufacturerEntity>("barrierManufacturers");

    let filter = bson::Document::new();

    let mut cursor = manufacturers.find(filter, None).await?;

    let mut manufacturers: Vec<BarrierManufacturerEntity> = Vec::new();
    while let Some(manufacturer) = cursor.next().await {
        manufacturers.push(manufacturer?);
    }

    Ok(manufacturers)
}

pub(crate) async fn get_barrier_manufacturer_by_id(
    db: &Database,
    id: &str,
) -> Result<Option<BarrierManufacturerEntity>> {
    let manufacturers = db.collection::<BarrierManufacturerEntity>("barrierManufacturers");

    let manufacturer = manufacturers
        .find_one(
            doc! {
              "_id": ObjectId::from_str(id)?
            },
            None,
        )
        .await?;

    Ok(manufacturer)
}

pub(crate) async fn get_manufacturers_by_id(
    db: &Database,
    ids: &[String],
) -> Result<Vec<BarrierManufacturerEntity>> {
    let manufacturers = db.collection::<BarrierManufacturerEntity>("barrierManufacturers");

    let ids = ids
        .iter()
        .map(|k| ObjectId::from_str(k).map_err(anyhow::Error::from))
        .collect::<Result<Vec<ObjectId>>>()?;

    let filter = doc! {
        "_id": {
            "$in": ids
        }
    };

    let mut cursor = manufacturers.find(filter, None).await?;

    let mut result: Vec<BarrierManufacturerEntity> = Vec::new();
    while let Some(manufacturer) = cursor.next().await {
        result.push(manufacturer?);
    }

    Ok(result)
}
