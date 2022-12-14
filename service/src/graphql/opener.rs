use crate::auth::Claims;
use crate::graphql::auth::{check_token, CheckTokenResult};
use crate::graphql::error::{Error, *};
use crate::graphql::simple_broker::SimpleBroker;
use crate::graphql::user::{NestedUserResult, UserLoader};
use crate::persistence::barrier_model::get_barrier_model_by_id;
use crate::persistence::opener::{
    create_opener, get_opener_by_id, get_opener_by_sn, get_openers, set_command_to_opener,
    update_opener, NewOpenerEntity, OpenerEntity, OpenerErrorEntity, UpdateOpenerEntity,
};
use crate::persistence::role::{get_role_by_id, RoleEntity};
use crate::server::OpenerServer;
use actix::prelude::*;
use async_graphql::dataloader::DataLoader;
use async_graphql::Context;
use async_graphql::*;
use futures::{Stream, StreamExt};
use mongodb::Database;
use std::convert::{TryFrom, TryInto};

use super::barrier_model::{BarrierModelLoader, NestedBarrierModelResult};

/// Describes statuses of commands for controller
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum CommandStatus {
    Ready,
    Pending,
    Success,
    Failed,
}

impl TryFrom<&str> for CommandStatus {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "READY" => Ok(CommandStatus::Ready),
            "PENDING" => Ok(CommandStatus::Pending),
            "SUCCESS" => Ok(CommandStatus::Success),
            "FAILED" => Ok(CommandStatus::Failed),
            _ => Err("Wrong command status"),
        }
    }
}

/// Describes types of commands for controller
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum CommandType {
    Info,
    Set,
    AddTags,
    RemoveTags,
    Update,
}

impl TryFrom<&str> for CommandType {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "INFO" => Ok(CommandType::Info),
            "SET" => Ok(CommandType::Set),
            "ADD_TAGS" => Ok(CommandType::AddTags),
            "REMOVE_TAGS" => Ok(CommandType::RemoveTags),
            "UPDATE" => Ok(CommandType::Update),
            _ => Err("Wrong command type"),
        }
    }
}

/// Describes error data returned by controller
#[derive(SimpleObject, Debug, Clone)]
pub(crate) struct OpenerError {
    pub serial_number: String,
    pub code: u32,
    pub description: String,
    pub details: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub(crate) struct Opener {
    id: ID,
    serial_number: String,
    version: Option<String>,
    alias: Option<String>,
    description: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    login: String,
    password: String,
    connected: bool,
    created_at: i64,
    updated_at: Option<i64>,
    last_error: Option<OpenerError>,
    last_command_type: Option<CommandType>,
    command_status: CommandStatus,

    #[graphql(skip)]
    nonce: Option<String>,

    #[graphql(skip)]
    barrier_model_id: Option<String>,

    #[graphql(skip)]
    user_id: Option<String>,
}

#[ComplexObject]
impl Opener {
    async fn owner(&self, ctx: &Context<'_>) -> Option<NestedUserResult> {
        self.user_id.as_ref()?;

        let user_id = self.user_id.as_ref().unwrap();

        let data_loader = ctx
            .data::<DataLoader<UserLoader>>()
            .expect("Can't get user data loader");

        let user = match data_loader.load_one(user_id.clone()).await {
            Err(e) => return Some(NestedUserResult::InternalServerError(e.message.into())),
            Ok(u) => u,
        };

        if user.is_none() {
            return Some(NestedUserResult::NotFoundError(NotFoundError::new(
                "Not found",
                "User",
            )));
        }

        Some(NestedUserResult::User(user.unwrap()))
    }

    async fn barrier_model(&self, ctx: &Context<'_>) -> Option<NestedBarrierModelResult> {
        self.barrier_model_id.as_ref()?;

        let barrier_model_id = self.barrier_model_id.as_ref().unwrap();

        let data_loader = ctx
            .data::<DataLoader<BarrierModelLoader>>()
            .expect("Can't get barrier model data loader");

        let model = match data_loader.load_one(barrier_model_id.clone()).await {
            Err(e) => {
                return Some(NestedBarrierModelResult::InternalServerError(
                    e.message.into(),
                ))
            }
            Ok(m) => m,
        };

        if model.is_none() {
            return Some(NestedBarrierModelResult::NotFoundError(NotFoundError::new(
                "Not found",
                "BarrierModel",
            )));
        }

        Some(NestedBarrierModelResult::BarrierModel(model.unwrap()))
    }
}

#[derive(Union)]
enum OpenerResult {
    Opener(Box<Opener>),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for OpenerResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => OpenerResult::InternalServerError(e),
            Error::UnauthorizedError(e) => OpenerResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => OpenerResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => OpenerResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to OpenerResult"),
        }
    }
}

#[derive(SimpleObject)]
struct Openers {
    items: Vec<Opener>,
}

#[derive(Union)]
enum OpenersResult {
    Openers(Openers),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for OpenersResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => OpenersResult::InternalServerError(e),
            Error::UnauthorizedError(e) => OpenersResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => OpenersResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => OpenersResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to OpenersResult"),
        }
    }
}

#[derive(Union)]
enum CreateOpenerResult {
    Opener(Box<Opener>),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
    AlreadyExistsError(AlreadyExistsError),
}

impl From<Error> for CreateOpenerResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => CreateOpenerResult::InternalServerError(e),
            Error::UnauthorizedError(e) => CreateOpenerResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => CreateOpenerResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => CreateOpenerResult::TokenIsExpiredError(e),
            Error::AlreadyExistsError(e) => CreateOpenerResult::AlreadyExistsError(e),
            _ => panic!("Can not cast from Error to CreateOpenerResult"),
        }
    }
}

#[derive(Union)]
enum UpdateOpenerResult {
    Opener(Box<Opener>),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
    NotFoundError(NotFoundError),
    IsInvalidError(IsInvalidError),
    NoUpdateDataProvidedError(NoUpdateDataProvidedError),
}

impl From<Error> for UpdateOpenerResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => UpdateOpenerResult::InternalServerError(e),
            Error::UnauthorizedError(e) => UpdateOpenerResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => UpdateOpenerResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => UpdateOpenerResult::TokenIsExpiredError(e),
            Error::NotFoundError(e) => UpdateOpenerResult::NotFoundError(e),
            Error::IsInvalidError(e) => UpdateOpenerResult::IsInvalidError(e),
            Error::NoUpdateDataProvidedError(e) => UpdateOpenerResult::NoUpdateDataProvidedError(e),
            _ => panic!("Can not cast from Error to UpdateOpenerResult"),
        }
    }
}

#[derive(Union)]
enum SetParamsCommandResult {
    Opener(Box<Opener>),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
    NotFoundError(NotFoundError),
    IsInvalidError(IsInvalidError),
    DeviceIsBusyError(DeviceIsBusyError),
    DeviceIsNotConnectedError(DeviceIsNotConnectedError),
    NoUpdateDataProvidedError(NoUpdateDataProvidedError),
}

impl From<Error> for SetParamsCommandResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => SetParamsCommandResult::InternalServerError(e),
            Error::UnauthorizedError(e) => SetParamsCommandResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => SetParamsCommandResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => SetParamsCommandResult::TokenIsExpiredError(e),
            Error::NotFoundError(e) => SetParamsCommandResult::NotFoundError(e),
            Error::IsInvalidError(e) => SetParamsCommandResult::IsInvalidError(e),
            Error::DeviceIsBusyError(e) => SetParamsCommandResult::DeviceIsBusyError(e),
            Error::DeviceIsNotConnectedError(e) => {
                SetParamsCommandResult::DeviceIsNotConnectedError(e)
            }
            Error::NoUpdateDataProvidedError(e) => {
                SetParamsCommandResult::NoUpdateDataProvidedError(e)
            }
            _ => panic!("Can not cast from Error to SetParamsCommandResult"),
        }
    }
}

fn user_id_default() -> Option<ID> {
    None
}

#[derive(InputObject)]
struct CreateOpenerInput {
    serial_number: String,
}

#[derive(InputObject)]
struct SetParamsCommandInput {
    barrier_model_id: Option<String>,
}

#[derive(InputObject)]
struct UpdateOpenerInput {
    alias: Option<String>,
    description: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    login: Option<String>,
    password: Option<String>,
}

fn check_update_data(new_opener: &UpdateOpenerInput) -> bool {
    new_opener.alias.is_some()
        || new_opener.description.is_some()
        || new_opener.lat.is_some()
        || new_opener.lng.is_some()
        || new_opener.login.is_some()
        || new_opener.password.is_some()
}

#[derive(Default)]
pub(super) struct OpenerMutation;

#[Object]
impl OpenerMutation {
    async fn create_opener(
        &self,
        ctx: &Context<'_>,
        opener: CreateOpenerInput,
    ) -> CreateOpenerResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.openers.create).await
        {
            return e.into();
        }

        let new_opener_entity = NewOpenerEntity {
            serial_number: opener.serial_number,
        };

        let opener = match create_opener(db, &new_opener_entity).await {
            Err(e) => {
                return match e.downcast_ref::<crate::persistence::error::Error>() {
                    Some(crate::persistence::error::Error::AlreadyExistsError) => {
                        CreateOpenerResult::AlreadyExistsError("Opener already exists".into())
                    }
                    None => CreateOpenerResult::InternalServerError(e.into()),
                }
            }
            Ok(opener) => opener,
        };

        let opener = match Opener::try_from(&opener) {
            Err(e) => {
                log::error!("Failed to convert opener {}", e);
                return CreateOpenerResult::InternalServerError(e.into());
            }
            Ok(m) => m,
        };

        CreateOpenerResult::Opener(Box::new(opener))
    }

    async fn update_opener(
        &self,
        ctx: &Context<'_>,
        serial_number: String,
        new_opener: UpdateOpenerInput,
        #[graphql(
            default_with = "user_id_default()",
            desc = "If admin wants to assign or change opener's user"
        )]
        user_id: Option<ID>,
    ) -> UpdateOpenerResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        let token: (&Claims, String) =
            match check_token(ctx, |role| role.access_rights.openers.edit).await {
                CheckTokenResult::Err(e) => return e.into(),
                CheckTokenResult::Ok { claims, role_name } => (claims, role_name),
            };

        if !check_update_data(&new_opener) {
            return UpdateOpenerResult::NoUpdateDataProvidedError("No update data provided".into());
        }

        // Non-admin users can't change opener's user
        if token.1 != "admin" && user_id.is_some() {
            return UpdateOpenerResult::PermissionDeniedError("Permission denied".into());
        }

        let opener = match get_opener_by_sn(db, &serial_number).await {
            Err(e) => return UpdateOpenerResult::InternalServerError(e.into()),
            Ok(o) => o,
        };

        if opener.is_none() {
            return UpdateOpenerResult::NotFoundError(NotFoundError::new("Not found", "Opener"));
        }

        let mut new_user_id: Option<String> = None;

        if let Some(opener_user_id) = opener.unwrap().user_id {
            // If non-admin user and user_id different it is error
            if opener_user_id.to_string() != token.0.user_id && token.1 != "admin" {
                return UpdateOpenerResult::PermissionDeniedError("Permission denied".into());
            }

            if token.1 == "admin" {
                // Change user if admin wants
                new_user_id = user_id.map(ID::into);
            }
        } else {
            // Admin must give user_id
            if token.1 == "admin" && user_id.is_none() {
                return UpdateOpenerResult::IsInvalidError(IsInvalidError::new(
                    "Invalid param",
                    "userId",
                ));
            }

            // Assign new user
            new_user_id = match token.1.as_str() {
                "admin" => user_id.map(ID::into),
                _ => Some(token.0.user_id.clone()),
            };
        }

        let new_opener_entity = UpdateOpenerEntity {
            user_id: new_user_id,
            alias: new_opener.alias,
            description: new_opener.description,
            lat: new_opener.lat,
            lng: new_opener.lng,
            login: new_opener.login,
            password: new_opener.password,
            nonce: None,
            version: None,
            connected: None,
            barrier_model_id: None,
        };

        let opener = match update_opener(db, &serial_number, &new_opener_entity).await {
            Err(e) => return UpdateOpenerResult::InternalServerError(e.into()),
            Ok(o) => o,
        };

        let opener = match Opener::try_from(&opener) {
            Err(e) => {
                log::error!("Failed to convert opener {}", e);
                return UpdateOpenerResult::InternalServerError(e.into());
            }
            Ok(o) => o,
        };

        UpdateOpenerResult::Opener(Box::new(opener))
    }

    async fn set_params_command(
        &self,
        ctx: &Context<'_>,
        id: ID,
        params: SetParamsCommandInput,
    ) -> SetParamsCommandResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.openers.edit).await
        {
            return e.into();
        }

        if params.barrier_model_id.is_none() {
            return SetParamsCommandResult::NoUpdateDataProvidedError(
                "No update data provided".into(),
            );
        }

        let opener = match get_opener_by_id(db, &id).await {
            Err(e) => return SetParamsCommandResult::InternalServerError(e.into()),
            Ok(o) => o,
        };

        if opener.is_none() {
            return SetParamsCommandResult::NotFoundError(NotFoundError::new(
                "Not found",
                "Opener",
            ));
        }

        let opener = opener.unwrap();

        let opener = match Opener::try_from(&opener) {
            Err(e) => {
                log::error!("Failed to convert opener {}", e);
                return SetParamsCommandResult::InternalServerError(e.into());
            }
            Ok(o) => o,
        };

        if !opener.connected {
            return SetParamsCommandResult::DeviceIsNotConnectedError(
                "Opener is not connected".into(),
            );
        }

        if opener.command_status == CommandStatus::Pending {
            return SetParamsCommandResult::DeviceIsBusyError("Opener is busy".into());
        }

        let is_new_model = opener.barrier_model_id != params.barrier_model_id;

        let command_status = if is_new_model { "PENDING" } else { "SUCCESS" };

        let last_command_type = "SET";

        if is_new_model {
            let model = match get_barrier_model_by_id(db, params.barrier_model_id.as_ref().unwrap())
                .await
            {
                Err(e) => return SetParamsCommandResult::InternalServerError(e.into()),
                Ok(m) => m,
            };

            if model.is_none() {
                return SetParamsCommandResult::NotFoundError(NotFoundError::new(
                    "Not found",
                    "BarrierModel",
                ));
            }

            let model = model.unwrap();

            let srv = ctx
                .data::<Addr<OpenerServer>>()
                .expect("Can't get opener server")
                .clone();

            let command = crate::server::message::SetCommand {
                login: opener.login.clone(),
                password: opener.password.clone(),
                nonce: opener.nonce.as_ref().unwrap().clone(),
                serial_number: opener.serial_number.clone(),
                barrier_model: params.barrier_model_id.as_ref().unwrap().clone(),
                barrier_algorithm: model.algorithm,
            };

            tokio::spawn(async move {
                if let Err(e) = srv.send(command).await {
                    log::error!("Failed to send set command to server: {}", e);
                }
            });
        }

        let opener = match set_command_to_opener(
            db,
            &opener.serial_number,
            command_status,
            last_command_type,
        )
        .await
        {
            Err(e) => return SetParamsCommandResult::InternalServerError(e.into()),
            Ok(o) => o,
        };

        log::info!("opener_entity: {:?}", opener);

        let opener = match Opener::try_from(&opener) {
            Err(e) => {
                log::error!("Failed to convert opener {}", e);
                return SetParamsCommandResult::InternalServerError(e.into());
            }
            Ok(o) => o,
        };

        SetParamsCommandResult::Opener(Box::new(opener))
    }
}

#[derive(Default)]
pub(super) struct OpenerQuery;

#[Object]
impl OpenerQuery {
    async fn opener(&self, ctx: &Context<'_>, id: ID) -> Option<OpenerResult> {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        let token: (&Claims, String) =
            match check_token(ctx, |role| role.access_rights.openers.view).await {
                CheckTokenResult::Err(e) => return Some(e.into()),
                CheckTokenResult::Ok { claims, role_name } => (claims, role_name),
            };

        let opener = match get_opener_by_id(db, &id).await {
            Err(e) => return Some(OpenerResult::InternalServerError(e.into())),
            Ok(o) => o,
        };

        opener.as_ref()?;

        let opener = opener.unwrap();

        // Admin can view any opener

        if token.1 == "admin" {
            let opener = match Opener::try_from(&opener) {
                Err(e) => {
                    log::error!("Failed to convert opener {}", e);
                    return Some(OpenerResult::InternalServerError(e.into()));
                }
                Ok(o) => o,
            };

            return Some(OpenerResult::Opener(Box::new(opener)));
        }

        // Others can view only theirs openers

        if let Some(user_id) = opener.user_id {
            if user_id.to_string() == token.0.user_id {
                let opener = match Opener::try_from(&opener) {
                    Err(e) => {
                        log::error!("Failed to convert opener {}", e);
                        return Some(OpenerResult::InternalServerError(e.into()));
                    }
                    Ok(o) => o,
                };

                return Some(OpenerResult::Opener(Box::new(opener)));
            }
        }

        Some(OpenerResult::PermissionDeniedError(
            "Permission denied".into(),
        ))
    }

    async fn openers(&self, ctx: &Context<'_>) -> OpenersResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        let token: (&Claims, String) =
            match check_token(ctx, |role| role.access_rights.openers.list).await {
                CheckTokenResult::Err(e) => return e.into(),
                CheckTokenResult::Ok { claims, role_name } => (claims, role_name),
            };

        let user_id = match token.1.as_str() {
            "admin" => None,
            _ => Some(&token.0.user_id),
        };

        let openers = match get_openers(db, user_id).await {
            Err(e) => {
                log::error!("Failed to get openers: {}", e.to_string());
                return OpenersResult::InternalServerError(e.into());
            }
            Ok(o) => o,
        };

        let openers = match openers.iter().map(Opener::try_from).collect() {
            Err(e) => {
                log::error!("Failed to convert openers: {}", e);
                return OpenersResult::InternalServerError(e.into());
            }
            Ok(o) => o,
        };

        OpenersResult::Openers(Openers { items: openers })
    }
}

impl TryFrom<&OpenerEntity> for Opener {
    type Error = &'static str;

    fn try_from(opener: &OpenerEntity) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ID::from(opener.id.unwrap()),
            serial_number: opener.serial_number.clone(),
            version: opener.version.clone(),
            alias: opener.alias.clone(),
            nonce: opener.nonce.clone(),
            description: opener.description.clone(),
            lat: opener.lat,
            lng: opener.lng,
            login: opener.login.clone(),
            password: opener.password.clone(),
            connected: opener.connected,
            created_at: opener.created_at.timestamp_millis(),
            updated_at: opener.updated_at.map(|t| t.timestamp_millis()),
            barrier_model_id: opener.barrier_model_id.map(|id| id.to_string()),
            user_id: opener.user_id.map(|id| id.to_string()),
            command_status: opener.command_status.as_str().try_into()?,
            last_command_type: opener
                .last_command_type
                .as_ref()
                .map(|t| t.as_str().try_into())
                .transpose()?,
            last_error: opener.last_error.as_ref().map(|e| e.into()),
        })
    }
}

impl From<&OpenerErrorEntity> for OpenerError {
    fn from(error: &OpenerErrorEntity) -> Self {
        Self {
            serial_number: error.serial_number.clone(),
            code: error.code,
            description: error.description.clone(),
            details: error.details.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OpenerConnectionChanged {
    pub serial_number: String,
    pub connected: bool,
    pub user_id: Option<String>,
}

#[Object]
impl OpenerConnectionChanged {
    async fn serial_number(&self) -> &String {
        &self.serial_number
    }

    async fn connected(&self) -> bool {
        self.connected
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OpenerCommandResult {
    pub serial_number: String,
    pub command_type: CommandType,
    pub command_status: CommandStatus,
    pub error: Option<OpenerError>,
    pub user_id: Option<String>,
}

#[Object]
impl OpenerCommandResult {
    async fn serial_number(&self) -> &String {
        &self.serial_number
    }

    async fn command_type(&self) -> &CommandType {
        &self.command_type
    }

    async fn command_status(&self) -> &CommandStatus {
        &self.command_status
    }

    async fn error(&self) -> &Option<OpenerError> {
        &self.error
    }
}

#[derive(Default)]
pub(super) struct OpenerSubscription;

#[Subscription]
impl OpenerSubscription {
    async fn opener_connection(
        &self,
        ctx: &Context<'_>,
        access_token: String,
    ) -> Result<impl Stream<Item = OpenerConnectionChanged>> {
        let claims = match crate::auth::decode_claims(&access_token) {
            Ok(claims) => claims,
            Err(_) => return Err("Unauthorized".into()),
        };

        let claims = match claims {
            Some(claims) => claims,
            None => return Err("Unauthorized".into()),
        };

        let db = ctx.data::<Database>().expect("Can't get db connection");

        let role = get_role_by_id(db, &claims.role_id).await;

        let role = match role {
            Ok(role) => role,
            Err(_) => return Err("Internal server error".into()),
        };

        let role = match role {
            Some(role) => role,
            None => return Err("Unauthorized".into()),
        };

        Ok(
            SimpleBroker::<OpenerConnectionChanged>::subscribe().filter(move |event| {
                log::info!("Event: {:?}", event);

                let res = check_user(&claims, &role, &event.user_id);

                async move { res }
            }),
        )
    }

    async fn opener_command(
        &self,
        ctx: &Context<'_>,
        serial_number: String,
        access_token: String,
    ) -> Result<impl Stream<Item = OpenerCommandResult> + '_> {
        let claims = match crate::auth::decode_claims(&access_token) {
            Ok(claims) => claims,
            Err(_) => return Err("Unauthorized".into()),
        };

        let claims = match claims {
            Some(claims) => claims,
            None => return Err("Unauthorized".into()),
        };

        let db = ctx
            .data::<Database>()
            .expect("Can't get db connection")
            .clone();

        let role = get_role_by_id(&db, &claims.role_id).await;

        let role = match role {
            Ok(role) => role,
            Err(_) => return Err("Internal server error".into()),
        };

        let role = match role {
            Some(role) => role,
            None => return Err("Unauthorized".into()),
        };

        Ok(
            SimpleBroker::<OpenerCommandResult>::subscribe().filter(move |event| {
                log::info!("Event: {:?}", event);

                let res = check_user(&claims, &role, &event.user_id);

                let serial_number = serial_number.clone();

                let db = db.clone();

                let same_opener = serial_number == event.serial_number;

                async move {
                    if !res || !same_opener {
                        return false;
                    }

                    let opener = get_opener_by_sn(&db, &serial_number).await;

                    match opener {
                        Err(_) => false,
                        Ok(opener) => opener.is_some(),
                    }
                }
            }),
        )
    }
}

fn check_user(claims: &Claims, role: &RoleEntity, user_id: &Option<String>) -> bool {
    if role.name == "admin" {
        true
    } else if role.name == "manufacturer" || user_id.is_none() {
        false
    } else {
        let user_id = user_id.as_ref().unwrap().clone();
        user_id == claims.user_id
    }
}
