use crate::auth::Claims;
use crate::graphql::auth::{check_token, CheckTokenResult};
use crate::graphql::error::{Error, *};
use crate::graphql::simple_broker::SimpleBroker;
use crate::graphql::user::{NestedUserResult, UserLoader};
use crate::persistence::opener::{
    create_opener, get_opener_by_id, get_opener_by_sn, get_openers, update_opener, NewOpenerEntity,
    OpenerEntity, UpdateOpenerEntity,
};
use crate::persistence::role::get_role_by_id;
use async_graphql::dataloader::DataLoader;
use async_graphql::*;
use futures::{Stream, StreamExt};
use mongodb::Database;

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
}

#[derive(Union)]
enum OpenerResult {
    Opener(Opener),
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
    Opener(Opener),
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
    Opener(Opener),
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
            _ => panic!("Can not cast from Error to CreateOpenerResult"),
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

        CreateOpenerResult::Opener(Opener::from(&opener))
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
        };

        let opener = match update_opener(db, &serial_number, &new_opener_entity).await {
            Err(e) => return UpdateOpenerResult::InternalServerError(e.into()),
            Ok(o) => o,
        };

        UpdateOpenerResult::Opener(Opener::from(&opener))
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
            return Some(OpenerResult::Opener(Opener::from(&opener)));
        }

        // Others can view only theirs openers

        if let Some(user_id) = opener.user_id {
            if user_id.to_string() == token.0.user_id {
                return Some(OpenerResult::Opener(Opener::from(&opener)));
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

        OpenersResult::Openers(Openers {
            items: openers.iter().map(Opener::from).collect(),
        })
    }
}

impl From<&OpenerEntity> for Opener {
    fn from(opener: &OpenerEntity) -> Self {
        Self {
            id: ID::from(opener.id.unwrap()),
            serial_number: opener.serial_number.clone(),
            version: opener.version.clone(),
            alias: opener.alias.clone(),
            description: opener.description.clone(),
            lat: opener.lat,
            lng: opener.lng,
            login: opener.login.clone(),
            password: opener.password.clone(),
            connected: opener.connected,
            created_at: opener.created_at.timestamp_millis(),
            updated_at: opener.updated_at.map(|t| t.timestamp_millis()),
            user_id: opener.user_id.map(|id| id.to_string()),
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

                let res = if role.name == "admin" {
                    true
                } else if role.name == "manufacturer" || event.user_id.is_none() {
                    false
                } else {
                    let user_id = event.user_id.as_ref().unwrap().clone();
                    user_id == claims.user_id
                };

                async move { res }
            }),
        )
    }
}
