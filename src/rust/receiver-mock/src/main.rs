#[allow(unused_imports)]
#[macro_use]
extern crate json_str;

use std::collections::HashMap;
use std::sync::Mutex;

use actix_service::Service;
use actix_web::web;

use chrono::Duration;
use clap::{value_t, App, Arg};

mod metrics;
mod options;
use options::Options;
mod router;
mod time;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let matches = App::new("Receiver mock")
      .version("0.0")
      .author("Dominik Rosiek <drosiek@sumologic.com>")
      .about("Receiver mock can be used for testing performance or functionality of kubernetes collection without sending data to sumologic")
      .arg(Arg::with_name("port")
          .short("p")
          .long("port")
          .value_name("port")
          .help("Port to listen on")
          .takes_value(true)
          .required(false))
      .arg(Arg::with_name("hostname")
          .short("l")
          .long("hostname")
          .value_name("hostname")
          .help("Hostname reported as the receiver. For kubernetes it will be '<service name>.<namespace>'")
          .takes_value(true)
          .required(false))
      .arg(Arg::with_name("print_logs")
          .short("r")
          .long("print-logs")
          .value_name("print_logs")
          .help("Use to print received logs on stdout")
          .takes_value(false)
          .required(false))
      .arg(Arg::with_name("print_headers")
          .long("print-headers")
          .value_name("print_headers")
          .help("Use to print received requests' headers")
          .takes_value(false)
          .required(false))
      .arg(Arg::with_name("print_metrics")
          .short("m")
          .long("print-metrics")
          .value_name("print_metrics")
          .help("Use to print received metrics (with dimensions) on stdout")
          .takes_value(false)
          .required(false))
      .get_matches();

    let port = value_t!(matches, "port", u16).unwrap_or(3000);
    let hostname = value_t!(matches, "hostname", String).unwrap_or("localhost".to_string());
    let opts = Options {
        print: options::Print {
            logs: matches.is_present("print_logs"),
            headers: matches.is_present("print_headers"),
            metrics: matches.is_present("print_metrics"),
        },
    };

    run_app(hostname, port, opts).await
}

async fn run_app(hostname: String, port: u16, opts: Options) -> std::io::Result<()> {
    let app_state = web::Data::new(router::AppState {
        metrics: Mutex::new(0),
        logs: Mutex::new(0),
        logs_bytes: Mutex::new(0),
        metrics_list: Mutex::new(HashMap::new()),
        metrics_ip_list: Mutex::new(HashMap::new()),
        logs_ip_list: Mutex::new(HashMap::new()),
    });

    let t = timer::Timer::new();
    // TODO: configure interval?
    // ref: https://github.com/SumoLogic/sumologic-kubernetes-tools/issues/59
    router::start_print_stats_timer(&t, Duration::seconds(60), app_state.clone()).ignore();

    let app_metadata = web::Data::new(router::AppMetadata {
        url: format!("http://{}:{}/receiver", hostname, port),
    });

    println!("Receiver mock is waiting for enemy on 0.0.0.0:{}!", port);
    let result = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            // Middleware printing headers for all handlers.
            // For a more robust middleware implementation (in its own type)
            // one can take a look at https://actix.rs/docs/middleware/
            .wrap_fn(move |req, srv| {
                if opts.print.headers {
                    let headers = req.headers();
                    router::print_request_headers(req.method(), req.version(), req.uri(), headers);
                }
                srv.call(req)
            })
            .app_data(app_state.clone()) // Mutable shared state
            .data(opts.clone())
            .route("/metrics-reset", web::post().to(router::handler_metrics_reset))
            .route("/metrics-list", web::get().to(router::handler_metrics_list))
            .route("/metrics-ips", web::get().to(router::handler_metrics_ips))
            .route("/metrics", web::get().to(router::handler_metrics))
            .service(
                web::scope("/terraform")
                    .app_data(app_metadata.clone())
                    .default_service(web::get().to(router::handler_terraform)),
            )
            // Treat every other url as receiver endpoint
            .default_service(web::get().to(router::handler_receiver))
                // Set metrics payload limit to 100MB
                .app_data(web::PayloadConfig::default().limit(100 * 2<<20))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await;

    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            eprintln!("server error: {}", e);
            Err(e)
        }
    }
}
