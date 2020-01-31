use nuclear_router::Router;
use regex::Regex;

#[test]
fn router_common() {
    let mut router: Router<usize> = Router::new();
    router
        .nest("/user/:user_id", |user| {
            user.insert("post/:post_id", 1)
                .insert("profile", 2)
                .insert("file/**", 3)
                .insert("", 4);
        })
        .insert("explore", 5)
        .nest("pan", |pan| {
            pan.insert("**", 6)
                .insert_regex(Regex::new(".*/(?P<name>.+)\\.php$").unwrap(), 7);
        });

    let cases: &[(_, _, &[(&str, &str)])] = &[
        (
            "/user/asd/post/123",
            1,
            &[("user_id", "asd"), ("post_id", "123")],
        ),
        ("/user/asd/profile", 2, &[("user_id", "asd")]),
        (
            "/user/asd/file/home/asd/.bashrc",
            3,
            &[("user_id", "asd"), ("**", "/home/asd/.bashrc")],
        ),
        ("/user/asd/", 4, &[("user_id", "asd")]),
        ("/explore", 5, &[]),
        ("/pan/home/asd", 6, &[("**", "/home/asd")]),
        ("/pan/phpinfo.php", 7, &[("name", "phpinfo")]),
    ];

    for (url, data, captures) in cases.iter().skip(5) {
        dbg!((url, data));
        let ret = router.find(url).unwrap();
        assert_eq!(ret.0, data);
        let v: Vec<(&str, &str)> = ret.1.collect();
        assert_eq!(&v, captures);
    }
}

#[test]
fn router_collision() {
    let mut router: Router<usize> = Router::new();
    router.try_insert("/u/:id/p/:id", 1).unwrap();
    router.try_insert("/u/:uid/p/:pid", 2).unwrap_err();

    let mut router: Router<usize> = Router::new();
    router.try_insert("/application/c/:a", 1).unwrap();
    router.try_insert("/application/b", 2).unwrap();
    router.try_insert("/application/b/:id", 3).unwrap();

    let mut router: Router<usize> = Router::new();
    router.try_insert("/application/**", 1).unwrap();
    router
        .try_nest("/application", |r| {
            r.insert("**", 2);
        })
        .unwrap_err();
}

#[test]
fn router_single() {
    let mut router: Router<usize> = Router::new();
    router.insert("/hello/:name", 1);

    assert_eq!(*router.find("/hello/world").unwrap().0, 1);
    assert!(router.find("/hello/world/asd").is_none());
    assert!(router.find("/hello").is_none());
}

#[test]
fn router_prefix() {
    let mut router: Router<usize> = Router::new();
    router.insert("/hello/world/:name", 1);
    router.insert("/hello/earth/", 2);
    router.insert("/asd", 3);

    assert!(router.find("/hello").is_none());
}