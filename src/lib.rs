use std::env;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

use crate::graphql::user::UserLoader;
use crate::session::WsOpenerSession;
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use actix_web_actors::ws;
use anyhow::Result;
use async_graphql::dataloader::DataLoader;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::Schema;
use async_graphql_actix_web::{Request, Response, WSSubscription};
use graphql::role::RoleLoader;
use mongodb::{options::ClientOptions, options::ResolverConfig, Database};

mod auth;
mod graphql;
mod persistence;
pub mod server;
mod session;

pub async fn init_db() -> Result<Database> {
    let client_uri =
        env::var("DATABASE_URL").expect("You must set the DATABASE_URL environment var!");

    let options =
        ClientOptions::parse_with_resolver_config(&client_uri, ResolverConfig::cloudflare())
            .await?;

    Ok(mongodb::Client::with_options(options)?.database("openers"))
}

pub async fn get_count(openers_count: web::Data<Arc<AtomicUsize>>) -> impl Responder {
    let current_count = openers_count.load(Ordering::SeqCst);
    format!("Connected openers count: {}", current_count)
}

pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::OpenerServer>>,
) -> Result<HttpResponse, Error> {
    log::info!("ws_route");
    ws::start(
        WsOpenerSession {
            id: None,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

pub async fn index_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/api").subscription_endpoint("/api/ws"),
        ))
}

pub fn create_schema_with_context(
    db: Database,
) -> Schema<graphql::Query, graphql::Mutation, graphql::Subscription> {
    let role_dataloader = DataLoader::new(RoleLoader { db: db.clone() }).max_batch_size(100);
    let user_dataloader = DataLoader::new(UserLoader { db: db.clone() }).max_batch_size(100);

    Schema::build(
        graphql::Query::default(),
        graphql::Mutation::default(),
        graphql::Subscription::default(),
    )
    .register_type::<graphql::Error>()
    .data(db)
    .data(role_dataloader)
    .data(user_dataloader)
    .finish()
}

pub async fn index_api(
    schema: web::Data<Schema<graphql::Query, graphql::Mutation, graphql::Subscription>>,
    http_req: HttpRequest,
    req: Request,
) -> Response {
    let mut query = req.into_inner();

    let claims = auth::get_claims(http_req);
    query = query.data(claims);

    schema.execute(query).await.into()
}

pub async fn index_subscriptions(
    schema: web::Data<Schema<graphql::Query, graphql::Mutation, graphql::Subscription>>,
    http_req: HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse> {
    log::info!("Subscribe");
    let schema = Schema::clone(&*schema);
    WSSubscription::start(schema, &http_req, payload)
}
