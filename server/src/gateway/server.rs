//! Implementation of the HTTP service

use super::config::Config;
use std::sync::Arc;

use crate::nodes::node::{CommandMessage, NodeManager, StopMessage};

use actix::{Actor, Addr, SystemService};
use actix_web::{error, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use auteur_controlling::controller::Command;
use tracing::error;

async fn create_command(
    node_manager: web::Data<Addr<NodeManager>>,
    json: web::Json<Command>,
) -> HttpResponse {
    let message = CommandMessage { command: json.0 };
    let response = node_manager.send(message).await;

    HttpResponse::Ok()
        .content_type::<_>("application/json")
        .body(format!("{:?}", response))
}

/// Start the server based on the passed `Config`.
pub async fn run(cfg: Config) -> Result<(), anyhow::Error> {
    let node_manager = NodeManager::from_registry();

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(node_manager.clone()))
            .wrap(actix_web::middleware::Logger::default())
            .wrap(tracing_actix_web::TracingLogger::default())
            .route("/command", web::post().to(create_command))
    });

    let server = if cfg.use_tls {
        use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
        builder.set_private_key_file(
            cfg.key_file.as_ref().expect("No key file given"),
            SslFiletype::PEM,
        )?;
        builder.set_certificate_chain_file(
            cfg.certificate_file
                .as_ref()
                .expect("No certificate file given"),
        )?;

        server.bind_openssl(format!("0.0.0.0:{}", cfg.port), builder)?
    } else {
        server.bind(format!("0.0.0.0:{}", cfg.port))?
    };

    server.run().await?;

    let _ = NodeManager::from_registry().send(StopMessage).await;

    Ok(())
}
