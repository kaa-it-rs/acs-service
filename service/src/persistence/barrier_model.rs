use anyhow::Result;
use bson::oid::ObjectId;
use futures::StreamExt;
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
    pub manufacturer_id: Option<ObjectId>,
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
