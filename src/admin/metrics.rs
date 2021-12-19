use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use opentelemetry::{
    global,
    metrics::{BoundCounter, BoundValueRecorder},
    KeyValue,
};
use opentelemetry_prometheus::PrometheusExporter;
use prometheus::{Encoder, TextEncoder};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::SystemTime;
use lazy_static::lazy_static;

use futures::future::*;
use garage_model::garage::Garage;
use garage_util::error::Error as GarageError;

// FIXME: This was on the example but not sure we need to be _that_ lazy
lazy_static! {
    static ref HANDLER_ALL: [KeyValue; 1] = [KeyValue::new("handler", "all")];
}

async fn serve_req(
    req: Request<Body>,
    state: Arc<AppState<'_>>,
) -> Result<Response<Body>, hyper::Error> {
    println!("Receiving request at path {}", req.uri());
    let request_start = SystemTime::now();

    state.http_counter.add(1);

    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = state.exporter.registry().gather();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            state.http_body_gauge.record(buffer.len() as u64);

            Response::builder()
                .status(200)
                .header(CONTENT_TYPE, encoder.format_type())
                .body(Body::from(buffer))
                .unwrap()
        }
        _ => Response::builder()
            .status(404)
            .body(Body::from("Not implemented"))
            .unwrap(),
    };

    state
        .http_req_histogram
        .record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()));
    Ok(response)
}

// AppState holds the metrics counter definition for Garage
// FIXME: we would rather have that split up among the different libraries?
struct AppState<'a> {
    exporter: PrometheusExporter,
    http_counter: BoundCounter<'a, u64>,
    http_body_gauge: BoundValueRecorder<'a, u64>,
    http_req_histogram: BoundValueRecorder<'a, f64>,
}

pub async fn run_admin_server(
 	garage: Arc<Garage>,
	shutdown_signal: impl Future<Output = ()>,
) -> Result<(), GarageError> {
	let exporter = opentelemetry_prometheus::exporter().init();

    let meter = global::meter("ex.com/hyper");
    let state = Arc::new(AppState {
        exporter,
        http_counter: meter
            .u64_counter("example.http_requests_total")
            .with_description("Total number of HTTP requests made.")
            .init()
            .bind(HANDLER_ALL.as_ref()),
        http_body_gauge: meter
            .u64_value_recorder("example.http_response_size_bytes")
            .with_description("The metrics HTTP response sizes in bytes.")
            .init()
            .bind(HANDLER_ALL.as_ref()),
        http_req_histogram: meter
            .f64_value_recorder("example.http_request_duration_seconds")
            .with_description("The HTTP request latencies in seconds.")
            .init()
            .bind(HANDLER_ALL.as_ref()),
    });

    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(move |_conn| {
        let state = state.clone();
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async move { Ok::<_, Infallible>(service_fn(move |req| serve_req(req, state.clone()))) }
    });

    let addr = &garage.config.admin_api.bind_addr;

    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(shutdown_signal);
    info!("Admin server listening on http://{}", addr);

    graceful.await?;
    Ok(())
}