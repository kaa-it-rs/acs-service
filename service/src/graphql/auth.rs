use super::error::Error;
use super::error::*;
use crate::auth::is_expired_error;
use crate::auth::Claims;
use crate::auth::{create_refresh_token, create_token};
use crate::persistence::role::get_role_by_id;
use crate::persistence::role::RoleEntity;
use crate::persistence::{client, user};
use anyhow::Result;
use async_graphql::*;
use mongodb::Database;
use opentelemetry::Context as OpenTelemetryContext;
use opentelemetry::{trace::TraceContextExt, KeyValue};

#[derive(SimpleObject)]
struct Credentials {
    /// Access token to use in protected requests (expired after 20 minutes)
    access_token: String,

    /// Refresh token to update access token after it's expiration (expired after 40 minutes)
    refresh_token: String,
}

#[derive(Union)]
enum AuthResult {
    Credentials(Credentials),
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
}

#[derive(Default)]
pub(super) struct AuthMutation;

#[Object]
impl AuthMutation {
    async fn login(&self, ctx: &Context<'_>, login: String, password: String) -> AuthResult {
        //let tracer = global::tracer("acs-service");

        let current_ctx = OpenTelemetryContext::current();
        let span = current_ctx.span();

        //let mut span = tracer.start("login");

        span.set_attribute(KeyValue::new("login", login.clone()));
        span.set_attribute(KeyValue::new("password", password.clone()));

        let db = ctx.data::<Database>().expect("Can't get db connection");

        let user = match user::get_user_by_login(db, &login).await {
            Ok(user) => user,
            Err(e) => return AuthResult::InternalServerError(e.into()),
        };

        println!("User: {:?}", user);

        if user.is_none() {
            return AuthResult::UnauthorizedError("Unauthorized".into());
        }

        let user = user.unwrap();

        match bcrypt::verify(password, &user.password) {
            Ok(res) => {
                if !res {
                    return AuthResult::UnauthorizedError("Unauthorized".into());
                }
            }

            Err(e) => return AuthResult::InternalServerError(e.into()),
        };

        let user_id = user.id.unwrap().to_string();

        let access_token = match create_token(user_id.clone(), user.role_id.to_string()) {
            Ok(token) => token,
            Err(e) => return AuthResult::InternalServerError(e.into()),
        };

        let refresh_token = create_refresh_token();

        if let Err(e) = client::create_client(db, refresh_token.clone(), &user_id).await {
            return AuthResult::InternalServerError(e.into());
        }

        span.add_event(
            "credentials",
            vec![
                KeyValue::new("accessToken", format!("Bearer {}", access_token)),
                KeyValue::new("refreshToken", refresh_token.clone()),
            ],
        );

        AuthResult::Credentials(Credentials {
            access_token: format!("Bearer {}", access_token),
            refresh_token,
        })
    }

    /// To get new access token and refresh token by refresh token
    async fn token(&self, ctx: &Context<'_>, refresh_token: String) -> AuthResult {
        let db = ctx.data::<Database>().expect("Can't get db connection");

        if let Err(e) = client::remove_expired_clients(db).await {
            return AuthResult::InternalServerError(e.into());
        }

        let client = match client::client(db, refresh_token.clone()).await {
            Ok(c) => c,
            Err(e) => return AuthResult::InternalServerError(e.into()),
        };

        if client.is_none() {
            return AuthResult::UnauthorizedError("Unauthorized".into());
        }

        let client = client.unwrap();
        let user_id = client.user_id.to_string();

        let user = match user::get_user_by_id(db, &user_id).await {
            Ok(u) => u,
            Err(e) => return AuthResult::InternalServerError(e.into()),
        };

        if user.is_none() {
            return AuthResult::UnauthorizedError("Unauthorized".into());
        }

        let user = user.unwrap();

        let access_token = match create_token(user_id, user.role_id.to_string()) {
            Ok(token) => token,
            Err(e) => return AuthResult::InternalServerError(e.into()),
        };

        let new_refresh_token = create_refresh_token();

        if let Err(e) = client::update_client(db, refresh_token, new_refresh_token.clone()).await {
            return AuthResult::InternalServerError(e.into());
        }

        AuthResult::Credentials(Credentials {
            access_token: format!("Bearer {}", access_token),
            refresh_token: new_refresh_token,
        })
    }
}

pub(super) enum CheckTokenResult<'a> {
    Ok {
        claims: &'a Claims,
        role_name: String,
    },
    Err(Error),
}

pub(super) async fn check_token<'a, F>(
    ctx: &Context<'a>,
    permission_checker: F,
) -> CheckTokenResult<'a>
where
    F: Fn(&RoleEntity) -> bool,
{
    let db = ctx.data::<Database>().expect("Can't get db connection");

    let claims = ctx.data::<Result<Option<Claims>>>().unwrap();

    let claims = match claims {
        Err(e) => {
            if is_expired_error(e) {
                return CheckTokenResult::Err(Error::TokenIsExpiredError(
                    "Token is expired".into(),
                ));
            }

            return CheckTokenResult::Err(Error::InternalServerError(e.into()));
        }
        Ok(claims) => {
            if claims.is_none() {
                println!("Claims is none");
                return CheckTokenResult::Err(Error::UnauthorizedError("Unauthorized".into()));
            }
            claims.as_ref().unwrap()
        }
    };

    let role = match get_role_by_id(db, &claims.role_id).await {
        Err(e) => return CheckTokenResult::Err(Error::InternalServerError(e.into())),
        Ok(r) => r,
    };

    if role.is_none() {
        return CheckTokenResult::Err(Error::InternalServerError("Role not found".into()));
    }

    let role = role.unwrap();

    if !permission_checker(&role) {
        return CheckTokenResult::Err(Error::PermissionDeniedError("Permission denied".into()));
    }

    CheckTokenResult::Ok {
        claims,
        role_name: role.name,
    }
}
