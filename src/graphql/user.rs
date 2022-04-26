use super::error::{Error, *};
use crate::graphql::auth::{check_token, CheckTokenResult};
use crate::graphql::role::{NestedRoleResult, RoleLoader};
use crate::persistence::user::{get_user_by_id, get_users_by_id, UserEntity};
use async_graphql::dataloader::{DataLoader, Loader};
use async_graphql::*;
use mongodb::Database;
use std::collections::HashMap;

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub(crate) struct User {
    id: ID,
    login: String,
    email: Option<String>,
    phone: Option<String>,
    created_at: i64,
    updated_at: Option<i64>,

    #[graphql(skip)]
    role_id: String,
}

#[ComplexObject]
impl User {
    async fn role(&self, ctx: &Context<'_>) -> NestedRoleResult {
        let data_loader = ctx
            .data::<DataLoader<RoleLoader>>()
            .expect("Can't get role data loader");

        let role = match data_loader.load_one(self.role_id.clone()).await {
            Err(e) => return NestedRoleResult::InternalServerError(e.message.into()),
            Ok(r) => r,
        };

        if role.is_none() {
            return NestedRoleResult::NotFoundError(NotFoundError::new("Not found", "Role"));
        }

        NestedRoleResult::Role(role.unwrap())
    }
}

#[derive(Union)]
enum UserResult {
    User(User),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for UserResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => UserResult::InternalServerError(e),
            Error::UnauthorizedError(e) => UserResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => UserResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => UserResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to RoleResult"),
        }
    }
}

#[derive(Union)]
pub(super) enum NestedUserResult {
    User(User),
    InternalServerError(InternalServerError),
    NotFoundError(NotFoundError),
}

#[derive(Default)]
pub(super) struct UserQuery;

#[Object]
impl UserQuery {
    async fn user(&self, ctx: &Context<'_>, id: ID) -> Option<UserResult> {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.users.view).await
        {
            return Some(e.into());
        }

        let user = match get_user_by_id(db, &id.to_string()).await {
            Err(e) => return Some(UserResult::InternalServerError(e.into())),
            Ok(u) => u,
        };

        if user.is_none() {
            return None;
        }

        let user = user.unwrap();

        Some(UserResult::User(User::from(&user)))
    }
}

impl From<&UserEntity> for User {
    fn from(user: &UserEntity) -> Self {
        Self {
            id: ID::from(user.id.unwrap()),
            login: user.login.clone(),
            email: user.email.clone(),
            phone: user.phone.clone(),
            created_at: user.created_at.timestamp_millis(),
            updated_at: user.updated_at.map(|t| t.timestamp_millis()),
            role_id: user.role_id.to_string(),
        }
    }
}

pub(crate) struct UserLoader {
    pub db: mongodb::Database,
}

#[async_trait::async_trait]
impl Loader<String> for UserLoader {
    type Value = User;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let users = get_users_by_id(&self.db, keys).await?;

        Ok(users
            .iter()
            .map(|user| (user.id.unwrap().to_string(), User::from(user)))
            .collect::<HashMap<_, _>>())
    }
}
