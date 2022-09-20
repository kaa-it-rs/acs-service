use anyhow::Result;
use bson::oid::ObjectId;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BarrierModelEntity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub algorithm: String,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<bson::DateTime>,

    #[serde(rename = "manufacturerId")]
    pub manufacturer_id: ObjectId,
}

pub(crate) async fn get_barrier_models(
    db: &Database,
    manufacturer_id: Option<String>,
) -> Result<Vec<BarrierModelEntity>> {
    log::info!("Get barrier models");

    let models = db.collection::<BarrierModelEntity>("barrierModels");

    let mut filter = bson::Document::new();

    if manufacturer_id.is_some() {
        filter.insert(
            "manufacturerId",
            ObjectId::from_str(manufacturer_id.unwrap().as_str())?,
        );
    }

    let mut cursor = models.find(filter, None).await?;

    let mut models: Vec<BarrierModelEntity> = Vec::new();
    while let Some(model) = cursor.next().await {
        models.push(model?);
    }

    Ok(models)
}

pub(crate) async fn get_barrier_model_by_id(
    db: &Database,
    id: &str,
) -> Result<Option<BarrierModelEntity>> {
    let models = db.collection::<BarrierModelEntity>("barrierModels");

    let model = models
        .find_one(
            doc! {
              "_id": ObjectId::from_str(id)?
            },
            None,
        )
        .await?;

    Ok(model)
}

pub(crate) async fn get_barrier_models_by_id(
    db: &Database,
    ids: &[String],
) -> Result<Vec<BarrierModelEntity>> {
    let models = db.collection::<BarrierModelEntity>("barrierModels");

    let ids = ids
        .iter()
        .map(|k| ObjectId::from_str(k).map_err(anyhow::Error::from))
        .collect::<Result<Vec<ObjectId>>>()?;

    let filter = doc! {
        "_id": {
            "$in": ids
        }
    };

    let mut cursor = models.find(filter, None).await?;

    let mut result: Vec<BarrierModelEntity> = Vec::new();
    while let Some(model) = cursor.next().await {
        result.push(model?);
    }

    Ok(result)
}
