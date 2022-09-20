use crate::graphql::auth::{check_token, CheckTokenResult};
use crate::graphql::error::{Error, *};
use crate::persistence::barrier_manufacturer::get_barrier_manufacturer_by_id;
use crate::persistence::barrier_manufacturer::get_barrier_manufacturers_by_id;
use crate::persistence::barrier_manufacturer::{
    get_barrier_manufacturers, BarrierManufacturerEntity,
};
use async_graphql::dataloader::{DataLoader, Loader};
use async_graphql::*;
use mongodb::Database;
use std::collections::HashMap;

use super::barrier_model::{BarrierModelLoader, BarrierModels, NestedBarrierModelsResult};

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct BarrierManufacturer {
    id: ID,
    name: String,
    created_at: i64,
    updated_at: Option<i64>,

    #[graphql(skip)]
    model_ids: Option<Vec<String>>,
}

#[ComplexObject]
impl BarrierManufacturer {
    async fn barrier_models(&self, ctx: &Context<'_>) -> NestedBarrierModelsResult {
        if self.model_ids.is_none() {
            return NestedBarrierModelsResult::NotFoundError(NotFoundError::new(
                "Not found",
                "BarrierModels",
            ));
        }

        let data_loader = ctx
            .data::<DataLoader<BarrierModelLoader>>()
            .expect("Can't get barrier model data loader");

        let models = match data_loader
            .load_many(self.model_ids.as_ref().unwrap().clone())
            .await
        {
            Err(e) => return NestedBarrierModelsResult::InternalServerError(e.message.into()),
            Ok(r) => r,
        };

        let models = models.into_values().collect();

        NestedBarrierModelsResult::BarrierModels(BarrierModels { items: models })
    }
}

#[derive(Union)]
enum BarrierManufacturerResult {
    BarrierManufacturer(BarrierManufacturer),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for BarrierManufacturerResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => BarrierManufacturerResult::InternalServerError(e),
            Error::UnauthorizedError(e) => BarrierManufacturerResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => BarrierManufacturerResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => BarrierManufacturerResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to BarrierManufacturerResult"),
        }
    }
}

#[derive(SimpleObject)]
struct BarrierManufacturers {
    items: Vec<BarrierManufacturer>,
}

#[derive(Union)]
enum BarrierManufacturersResult {
    BarrierManufacturers(BarrierManufacturers),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for BarrierManufacturersResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => BarrierManufacturersResult::InternalServerError(e),
            Error::UnauthorizedError(e) => BarrierManufacturersResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => BarrierManufacturersResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => BarrierManufacturersResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to BarrierManufacturersResult"),
        }
    }
}

#[derive(Union)]
pub(super) enum NestedBarrierManufacturerResult {
    BarrierManufacturer(BarrierManufacturer),
    InternalServerError(InternalServerError),
    NotFoundError(NotFoundError),
}

#[derive(Default)]
pub(super) struct BarrierManufacturerQuery;

#[Object]
impl BarrierManufacturerQuery {
    async fn barrier_manufacturer(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Option<BarrierManufacturerResult> {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.barrier_manufacturers.view).await
        {
            return Some(e.into());
        }

        let manufacturer = match get_barrier_manufacturer_by_id(db, &id).await {
            Err(e) => return Some(BarrierManufacturerResult::InternalServerError(e.into())),
            Ok(m) => m,
        };

        manufacturer.as_ref()?;

        let manufacturer = manufacturer.unwrap();

        Some(BarrierManufacturerResult::BarrierManufacturer(
            BarrierManufacturer::from(&manufacturer),
        ))
    }

    async fn barrier_manufacturers(&self, ctx: &Context<'_>) -> BarrierManufacturersResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.barrier_manufacturers.list).await
        {
            return e.into();
        }

        let manufacturers = match get_barrier_manufacturers(db).await {
            Err(e) => {
                log::error!("Failed to get barrier manufacturers: {}", e.to_string());
                return BarrierManufacturersResult::InternalServerError(e.into());
            }
            Ok(m) => m,
        };

        BarrierManufacturersResult::BarrierManufacturers(BarrierManufacturers {
            items: manufacturers
                .iter()
                .map(BarrierManufacturer::from)
                .collect(),
        })
    }
}

impl From<&BarrierManufacturerEntity> for BarrierManufacturer {
    fn from(manufacturer: &BarrierManufacturerEntity) -> Self {
        Self {
            id: ID::from(manufacturer.id.unwrap()),
            name: manufacturer.name.clone(),
            created_at: manufacturer.created_at.timestamp_millis(),
            updated_at: manufacturer.updated_at.map(|t| t.timestamp_millis()),
            model_ids: manufacturer
                .model_ids
                .as_ref()
                .map(|v| v.iter().map(|id| id.to_string()).collect()),
        }
    }
}

pub(crate) struct BarrierManufacturerLoader {
    pub db: mongodb::Database,
}

#[async_trait::async_trait]
impl Loader<String> for BarrierManufacturerLoader {
    type Value = BarrierManufacturer;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let manufacturers = get_barrier_manufacturers_by_id(&self.db, keys).await?;

        Ok(manufacturers
            .iter()
            .map(|manufacturer| {
                (
                    manufacturer.id.unwrap().to_string(),
                    BarrierManufacturer::from(manufacturer),
                )
            })
            .collect::<HashMap<_, _>>())
    }
}
