use crate::graphql::auth::{check_token, CheckTokenResult};
use crate::graphql::barrier_manufacturer::BarrierManufacturerLoader;
use crate::graphql::barrier_manufacturer::NestedBarrierManufacturerResult;
use crate::graphql::error::{Error, *};
use crate::persistence::barrier_model::get_barrier_models;
use crate::persistence::barrier_model::BarrierModelEntity;
use crate::persistence::barrier_model::{get_barrier_model_by_id, get_barrier_models_by_id};
use async_graphql::dataloader::{DataLoader, Loader};
use async_graphql::*;
use mongodb::Database;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;

/// Algorithms supported by barriers
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BarrierAlgorithm {
    /// Algorithm with impulse for open and close at the same pin
    OpenClose,

    /// Algorithm with one impulse only for open
    Open,

    /// Algorithm with impulse for open and close at two different pins
    TwoDoors,
}

impl TryFrom<&str> for BarrierAlgorithm {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "OPEN_CLOSE" => Ok(BarrierAlgorithm::OpenClose),
            "OPEN" => Ok(BarrierAlgorithm::Open),
            "TWO_DOORS" => Ok(BarrierAlgorithm::TwoDoors),
            _ => Err("Wrong barrier model"),
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct BarrierModel {
    id: ID,
    name: String,
    algorithm: BarrierAlgorithm,
    created_at: i64,
    updated_at: Option<i64>,

    #[graphql(skip)]
    manufacturer_id: String,
}

#[ComplexObject]
impl BarrierModel {
    async fn barrier_manufacturer(&self, ctx: &Context<'_>) -> NestedBarrierManufacturerResult {
        let data_loader = ctx
            .data::<DataLoader<BarrierManufacturerLoader>>()
            .expect("Can't get barrier manufacturer data loader");

        let manufacturer = match data_loader.load_one(self.manufacturer_id.clone()).await {
            Err(e) => {
                return NestedBarrierManufacturerResult::InternalServerError(e.message.into())
            }
            Ok(r) => r,
        };

        if manufacturer.is_none() {
            return NestedBarrierManufacturerResult::NotFoundError(NotFoundError::new(
                "Not found",
                "BarrierManufacturer",
            ));
        }

        NestedBarrierManufacturerResult::BarrierManufacturer(manufacturer.unwrap())
    }
}

fn manufacturer_id_default() -> Option<ID> {
    None
}

#[derive(Union)]
enum BarrierModelResult {
    BarrierModel(BarrierModel),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for BarrierModelResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => BarrierModelResult::InternalServerError(e),
            Error::UnauthorizedError(e) => BarrierModelResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => BarrierModelResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => BarrierModelResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to BarrierModelResult"),
        }
    }
}

#[derive(SimpleObject)]
pub(super) struct BarrierModels {
    pub items: Vec<BarrierModel>,
}

#[derive(Union)]
enum BarrierModelsResult {
    BarrierModels(BarrierModels),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for BarrierModelsResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => BarrierModelsResult::InternalServerError(e),
            Error::UnauthorizedError(e) => BarrierModelsResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => BarrierModelsResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => BarrierModelsResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to BarrierModelsResult"),
        }
    }
}

#[derive(Union)]
pub(super) enum NestedBarrierModelsResult {
    BarrierModels(BarrierModels),
    InternalServerError(InternalServerError),
    NotFoundError(NotFoundError),
}

#[derive(Union)]
pub(super) enum NestedBarrierModelResult {
    BarrierModel(BarrierModel),
    InternalServerError(InternalServerError),
    NotFoundError(NotFoundError),
}

#[derive(Default)]
pub(super) struct BarrierModelQuery;

#[Object]
impl BarrierModelQuery {
    async fn barrier_model(&self, ctx: &Context<'_>, id: ID) -> Option<BarrierModelResult> {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.barrier_models.view).await
        {
            return Some(e.into());
        }

        let model = match get_barrier_model_by_id(db, &id).await {
            Err(e) => return Some(BarrierModelResult::InternalServerError(e.into())),
            Ok(m) => m,
        };

        model.as_ref()?;

        let model = model.unwrap();

        let model = match BarrierModel::try_from(&model) {
            Err(e) => {
                log::error!("Failed to convert barrier model {}", e);
                return Some(BarrierModelResult::InternalServerError(e.into()));
            }
            Ok(m) => m,
        };

        Some(BarrierModelResult::BarrierModel(model))
    }

    async fn barrier_models(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            default_with = "manufacturer_id_default()",
            desc = "For filter barrier models by manufacturer"
        )]
        manufacturer_id: Option<ID>,
    ) -> BarrierModelsResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.barrier_models.list).await
        {
            return e.into();
        }

        let new_manufacturer_id: Option<String> = manufacturer_id.map(ID::into);

        let models = match get_barrier_models(db, new_manufacturer_id).await {
            Err(e) => {
                log::error!("Failed to get barrier models: {}", e.to_string());
                return BarrierModelsResult::InternalServerError(e.into());
            }
            Ok(o) => o,
        };

        let models = match models.iter().map(BarrierModel::try_from).collect() {
            Err(e) => {
                log::error!("Failed to convert barrier models: {}", e);
                return BarrierModelsResult::InternalServerError(e.into());
            }
            Ok(m) => m,
        };

        BarrierModelsResult::BarrierModels(BarrierModels { items: models })
    }
}

impl TryFrom<&BarrierModelEntity> for BarrierModel {
    type Error = &'static str;

    fn try_from(model: &BarrierModelEntity) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ID::from(model.id.unwrap()),
            name: model.name.clone(),
            algorithm: model.algorithm.as_str().try_into()?,
            created_at: model.created_at.timestamp_millis(),
            updated_at: model.updated_at.map(|t| t.timestamp_millis()),
            manufacturer_id: model.manufacturer_id.to_string(),
        })
    }
}

pub(crate) struct BarrierModelLoader {
    pub db: mongodb::Database,
}

#[async_trait::async_trait]
impl Loader<String> for BarrierModelLoader {
    type Value = BarrierModel;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let models = get_barrier_models_by_id(&self.db, keys).await?;

        let models: Vec<BarrierModel> = match models.iter().map(BarrierModel::try_from).collect() {
            Err(e) => {
                log::error!("Failed to convert barrier models: {}", e);
                return Err(e.into());
            }
            Ok(m) => m,
        };

        Ok(models
            .into_iter()
            .map(|model| (model.id.to_string(), model))
            .collect::<HashMap<_, _>>())
    }
}
