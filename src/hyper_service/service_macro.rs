#[macro_export]
macro_rules! router_service {
    {$($method:tt $pattern:expr => $data:expr),+ ; _ => $default:expr} => {{
        let mut __router = $crate::HttpRouter::new();
        $(router_service!(@entry __router, $method, $pattern, $data);)+
        __router.with_default($default)
    }};

    {$($method:tt $pattern:expr => $data:expr),+} => {{
        let mut __router = $crate::HttpRouter::new();
        $(router_service!(@entry __router, $method, $pattern, $data);)+
        __router
    }};

    {@entry $router:expr, @, $prefix:expr, $sub_router:expr} => {
        $router.nest($prefix, |__r| *__r = $sub_router)
    };
    {@entry $router:expr, GET, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::GET, $pattern, $data)
    };
    {@entry $router:expr, POST, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::POST, $pattern, $data)
    };
    {@entry $router:expr, PUT, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::PUT, $pattern, $data)
    };
    {@entry $router:expr, DELETE, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::DELETE, $pattern, $data)
    };
    {@entry $router:expr, HEAD, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::HEAD, $pattern, $data)
    };
    {@entry $router:expr, OPTIONS, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::OPTIONS, $pattern, $data)
    };
    {@entry $router:expr, CONNECT, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::CONNECT, $pattern, $data)
    };
    {@entry $router:expr, PATCH, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::PATCH, $pattern, $data)
    };
    {@entry $router:expr, TRACE, $pattern:expr, $data:expr} => {
        $router.route($crate::Method::TRACE, $pattern, $data)
    };
}
