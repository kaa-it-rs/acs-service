use crate::graphql::barrier_manufacturer::BarrierManufacturerLoader;
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
use async_graphql::extensions::OpenTelemetry;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::Schema;
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use graphql::barrier_model::BarrierModelLoader;
use graphql::role::RoleLoader;
use mongodb::{options::ClientOptions, options::ResolverConfig, Database};
use opentelemetry::global;

mod auth;
mod graphql;
mod persistence;
pub mod server;
mod session;

pub fn init_tracer() -> Result<()> {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    _ = opentelemetry_jaeger::new_pipeline()
        .with_agent_endpoint("jaeger:6831")
        .with_service_name("acs-service")
        .with_max_packet_size(9_216)
        //.with_auto_split_batch(true)
        //.install_batch(opentelemetry::runtime::AsyncStd)
        .install_simple()
        .unwrap();

    Ok(())
}

pub fn shutdown_tracer() {
    global::shutdown_tracer_provider();
}

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

pub async fn index_playground() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/api").subscription_endpoint("/api/ws"),
        )))
}

pub fn create_schema_with_context(
    db: Database,
) -> Schema<graphql::Query, graphql::Mutation, graphql::Subscription> {
    let role_dataloader =
        DataLoader::new(RoleLoader { db: db.clone() }, async_std::task::spawn).max_batch_size(100);
    let user_dataloader =
        DataLoader::new(UserLoader { db: db.clone() }, async_std::task::spawn).max_batch_size(100);
    let barrier_manufacturer_dataloader = DataLoader::new(
        BarrierManufacturerLoader { db: db.clone() },
        async_std::task::spawn,
    )
    .max_batch_size(100);
    let barrier_model_dataloader = DataLoader::new(
        BarrierModelLoader { db: db.clone() },
        async_std::task::spawn,
    )
    .max_batch_size(100);

    let tracer = global::tracer("acs_service");

    let otel = OpenTelemetry::<global::BoxedTracer>::new(tracer);

    Schema::build(
        graphql::Query::default(),
        graphql::Mutation::default(),
        graphql::Subscription::default(),
    )
    .register_output_type::<graphql::Error>()
    .data(db)
    .data(role_dataloader)
    .data(user_dataloader)
    .data(barrier_manufacturer_dataloader)
    .data(barrier_model_dataloader)
    .extension(otel)
    .finish()
}

pub async fn index_api(
    schema: web::Data<Schema<graphql::Query, graphql::Mutation, graphql::Subscription>>,
    http_req: HttpRequest,
    req: GraphQLRequest,
) -> GraphQLResponse {
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
    GraphQLSubscription::new(schema).start(&http_req, payload)
}
