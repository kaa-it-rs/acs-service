use super::auth::check_token;
use super::error::Error;
use super::error::*;
use crate::graphql::auth::CheckTokenResult;
use crate::persistence::role::{get_role_by_id, get_roles_by_id, RoleEntity};
use async_graphql::dataloader::Loader;
use async_graphql::*;
use mongodb::Database;
use std::collections::HashMap;

#[derive(SimpleObject, Clone)]
pub(crate) struct UsersRights {
    /// Can view the list of users
    list: bool,

    /// Can view user details
    view: bool,

    /// Can create user
    create: bool,

    /// Can edit user
    edit: bool,

    /// Can delete user
    delete: bool,
}

#[derive(SimpleObject, Clone)]
pub(crate) struct RolesRights {
    /// Can view the list of roles
    list: bool,

    /// Can view role details
    view: bool,

    /// Can create role
    create: bool,

    /// Can edit role
    edit: bool,

    /// Can delete role
    delete: bool,
}

#[derive(SimpleObject, Clone)]
pub(crate) struct OpenersRights {
    /// Can view the list of openers
    list: bool,

    /// Can view opener details
    view: bool,

    /// Can create opener
    create: bool,

    /// Can edit opener
    edit: bool,

    /// Can delete opener
    delete: bool,
}

#[derive(SimpleObject, Clone)]
pub(crate) struct AccessRights {
    users: UsersRights,
    roles: RolesRights,
    openers: OpenersRights,
}

#[derive(SimpleObject, Clone)]
pub(crate) struct Role {
    id: ID,
    name: String,
    access_rights: AccessRights,
    created_at: i64,
}

#[derive(Union)]
enum RoleResult {
    Role(Role),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
}

impl From<Error> for RoleResult {
    fn from(e: Error) -> Self {
        match e {
            Error::InternalServerError(e) => RoleResult::InternalServerError(e),
            Error::UnauthorizedError(e) => RoleResult::UnauthorizedError(e),
            Error::PermissionDeniedError(e) => RoleResult::PermissionDeniedError(e),
            Error::TokenIsExpiredError(e) => RoleResult::TokenIsExpiredError(e),
            _ => panic!("Can not cast from Error to RoleResult"),
        }
    }
}

#[derive(Union)]
pub(super) enum NestedRoleResult {
    Role(Role),
    InternalServerError(InternalServerError),
    NotFoundError(NotFoundError),
}

#[derive(Default)]
pub(super) struct RoleQuery;

#[Object]
impl RoleQuery {
    async fn role(&self, ctx: &Context<'_>, id: ID) -> Option<RoleResult> {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let CheckTokenResult::Err(e) =
            check_token(ctx, |role| role.access_rights.roles.view).await
        {
            return Some(e.into());
        }

        let role = match get_role_by_id(db, &id.to_string()).await {
            Err(e) => return Some(RoleResult::InternalServerError(e.into())),
            Ok(r) => r,
        };

        if role.is_none() {
            return None;
        }

        let role = role.unwrap();

        Some(RoleResult::Role(Role::from(&role)))
    }
}

impl From<&RoleEntity> for Role {
    fn from(role: &RoleEntity) -> Self {
        Self {
            id: ID::from(role.id.unwrap()),
            name: role.name.clone(),
            created_at: role.created_at.timestamp_millis(),
            access_rights: AccessRights {
                users: UsersRights {
                    list: role.access_rights.users.list,
                    view: role.access_rights.users.view,
                    create: role.access_rights.users.create,
                    edit: role.access_rights.users.edit,
                    delete: role.access_rights.users.delete,
                },
                roles: RolesRights {
                    list: role.access_rights.roles.list,
                    view: role.access_rights.roles.view,
                    create: role.access_rights.roles.create,
                    edit: role.access_rights.roles.edit,
                    delete: role.access_rights.roles.delete,
                },
                openers: OpenersRights {
                    list: role.access_rights.openers.list,
                    view: role.access_rights.openers.view,
                    create: role.access_rights.openers.create,
                    edit: role.access_rights.openers.edit,
                    delete: role.access_rights.openers.delete,
                },
            },
        }
    }
}

pub(crate) struct RoleLoader {
    pub db: mongodb::Database,
}

#[async_trait::async_trait]
impl Loader<String> for RoleLoader {
    type Value = Role;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let roles = get_roles_by_id(&self.db, keys).await?;

        Ok(roles
            .iter()
            .map(|role| (role.id.unwrap().to_string(), Role::from(role)))
            .collect::<HashMap<_, _>>())
    }
}
