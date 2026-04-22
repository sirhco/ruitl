//! Server-side streaming demo.
//!
//! Serves a large HTML document in chunks by breaking a `Html::Fragment` at
//! its top-level children and piping each piece through `hyper::Body::wrap_stream`.
//! Observe via `curl --no-buffer http://localhost:3000/big`.
//!
//! Run: `cargo run --example streaming_demo`

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use ruitl::html::*;
use std::convert::Infallible;
use std::net::SocketAddr;

fn big_page() -> Html {
    let mut children: Vec<Html> = Vec::with_capacity(102);
    children.push(Html::Raw("<!DOCTYPE html>\n".to_string()));
    children.push(Html::Element(
        HtmlElement::new("head")
            .child(Html::Element(HtmlElement::new("title").text("Streaming"))),
    ));
    children.push(Html::Raw("<body>\n".to_string()));
    for i in 0..100 {
        children.push(Html::Element(
            HtmlElement::new("section")
                .attr("data-i", &format!("{i}"))
                .child(Html::Element(
                    HtmlElement::new("h2").text(&format!("Section {i}")),
                ))
                .child(Html::Element(HtmlElement::new("p").text(
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                ))),
        ));
    }
    children.push(Html::Raw("</body>\n".to_string()));
    Html::Fragment(children)
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let msg = "streaming demo — hit /big for a chunked response";
            Ok(Response::new(Body::from(msg)))
        }
        (&Method::GET, "/big") => {
            // Split into per-section chunks. Each chunk is pushed over the wire
            // as it becomes ready. `curl --no-buffer` surfaces the effect.
            let chunks = big_page().to_chunks();
            let stream = futures::stream::iter(
                chunks
                    .into_iter()
                    .map(|c| Ok::<_, std::io::Error>(hyper::body::Bytes::from(c))),
            );
            let body = Body::wrap_stream(stream);
            Ok(Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(body)
                .unwrap())
        }
        _ => {
            let mut r = Response::new(Body::from("not found"));
            *r.status_mut() = StatusCode::NOT_FOUND;
            Ok(r)
        }
    }
}

#[tokio::main]
async fn main() {
    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();
    let make_svc = make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_svc);
    println!("streaming_demo listening on http://{addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}
