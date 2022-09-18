use std::sync::{atomic::AtomicUsize, Arc};

use actix::*;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{guard, web, App, HttpServer};
use anyhow::Result;
use env_logger::Env;

use acs_service::server::OpenerServer;
use acs_service::{
    create_schema_with_context, get_count, index_api, index_playground, index_subscriptions,
    init_db, init_tracer, shutdown_tracer, ws_route,
};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    _ = init_tracer();

    log::info!("dfsdfsdfdsfxxорлорлmhgfhgfhgfhbgdhgfh");
    let db = init_db().await?;

    let schema = create_schema_with_context(db.clone());

    let openers_count = Arc::new(AtomicUsize::new(0));

    let opener_server = OpenerServer::new(openers_count.clone(), db.clone()).start();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .app_data(web::Data::new(openers_count.clone()))
            .app_data(web::Data::new(opener_server.clone()))
            .app_data(web::Data::new(schema.clone()))
            .route("/playground", web::get().to(index_playground))
            .service(web::resource("/api").guard(guard::Post()).to(index_api))
            .service(
                web::resource("/api/ws")
                    .guard(guard::Get())
                    .guard(guard::Header("upgrade", "websocket"))
                    .to(index_subscriptions),
            )
            .route("/count/", web::get().to(get_count))
            .service(web::resource("/ws").to(ws_route))
    })
    .bind("0.0.0.0:4000")?
    .run()
    .await?;

    shutdown_tracer();

    Ok(())
}
