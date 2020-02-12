use nuclear_router::{router_service, OwnedCaptures};

use std::convert::Infallible as Never;

use hyper::service::make_service_fn;
use hyper::{Body, Request, Response};

async fn not_found(req: Request<Body>, _: OwnedCaptures) -> Result<Response<Body>, Never> {
    dbg!((req.method(), req.uri().path()));
    let res = hyper::Response::builder()
        .status(404)
        .body(hyper::Body::from("404 Not Found"))
        .unwrap();
    Ok(res)
}

async fn hello(_: Request<Body>, params: OwnedCaptures) -> Result<Response<Body>, Never> {
    let name = params.get("name").unwrap();
    dbg!(name);
    Ok(Response::new(Body::from(format!("hello, {}!", name))))
}

async fn file(_: Request<Body>, params: OwnedCaptures) -> Result<Response<Body>, Never> {
    let path = params.get("filepath").unwrap();
    dbg!(path);
    Ok(Response::new(Body::from(format!("access file: {}", path))))
}

#[tokio::main]
async fn main() {
    let router = router_service! {
        GET "/hello/:name" => hello,
        @ "/api/v1" => router_service!{
            GET "/file/*filepath" => file
        };
        _ => not_found
    }
    .into_shared();

    let make = make_service_fn(|_| {
        let new_router = router.clone();
        async move { Ok::<_, Never>(new_router) }
    });

    let addr = "127.0.0.1:3000";

    let server = hyper::Server::bind(&addr.parse().unwrap()).serve(make);

    println!("Server is listening on: http://{}", addr);
    println!("hello: http://{}/hello/world", addr);
    println!("api: http://{}/api/v1/file/path/to/public/file", addr);
    println!("404: http://{}/other/path", addr);
    println!();

    server.await.unwrap();
}
