#[cfg(feature = "http-router")]
#[test]
fn http_router_macro() {
    use nuclear_router::{http_router, HttpRouter, Method};

    let router: HttpRouter<i32> = http_router! {
        GET "/u/:uid/p/:pid" => 1i32,
        POST "/u/:uid/p" => 2,
        @ "/v1" => http_router!{
            GET "/info" => 3_i32,
            POST "/info" => 4,
            @ "/u/:uid" => http_router!{
                GET "p/:pid" => 6,
                POST "p" => 7
            }
        },
        HEAD "**" => 5
    };

    assert_eq!(*router.find(&Method::GET, "/u/asd/p/qwe").unwrap().0, 1);
    assert_eq!(*router.find(&Method::POST, "/u/asd/p").unwrap().0, 2);
    assert_eq!(*router.find(&Method::GET, "/v1/info").unwrap().0, 3);
    assert_eq!(*router.find(&Method::POST, "/v1/info").unwrap().0, 4);
    assert_eq!(*router.find(&Method::HEAD, "/home/asd").unwrap().0, 5);
    assert_eq!(*router.find(&Method::GET, "/v1/u/asd/p/qwe").unwrap().0, 6);
    assert_eq!(*router.find(&Method::POST, "/v1/u/asd/p").unwrap().0, 7);
}
