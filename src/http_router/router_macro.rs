#[macro_export]
macro_rules! http_router {
    {$($method:tt $pattern:expr => $data:expr),+} => {{
        let mut __router = $crate::HttpRouter::new();
        $(http_router!(@entry __router, $method, $pattern, $data);)+
        __router
    }};

    {@entry $router:expr, @, $prefix:expr, $sub_router:expr} => {
        $router.insert_router($prefix, $sub_router)
    };
    {@entry $router:expr, GET, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::GET, $pattern, $data)
    };
    {@entry $router:expr, POST, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::POST, $pattern, $data)
    };
    {@entry $router:expr, PUT, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::PUT, $pattern, $data)
    };
    {@entry $router:expr, DELETE, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::DELETE, $pattern, $data)
    };
    {@entry $router:expr, HEAD, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::HEAD, $pattern, $data)
    };
    {@entry $router:expr, OPTIONS, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::OPTIONS, $pattern, $data)
    };
    {@entry $router:expr, CONNECT, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::CONNECT, $pattern, $data)
    };
    {@entry $router:expr, PATCH, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::PATCH, $pattern, $data)
    };
    {@entry $router:expr, TRACE, $pattern:expr, $data:expr} => {
        $router.insert($crate::Method::TRACE, $pattern, $data)
    };
}
