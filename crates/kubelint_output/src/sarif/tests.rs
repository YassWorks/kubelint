use super::{is_relative, uri};

#[test]
fn uris() {
    assert_eq!(uri("deploy/web.yaml"), "deploy/web.yaml");
    assert_eq!(uri("my dir/é.yaml"), "my%20dir/%C3%A9.yaml");
    assert_eq!(uri("<stdin>"), "%3Cstdin%3E");
    assert_eq!(uri("a:b.yaml"), "a%3Ab.yaml");
    assert_eq!(uri("/srv/app/x.yaml"), "file:///srv/app/x.yaml");
    assert_eq!(uri("C:/work/x.yaml"), "file:///C:/work/x.yaml");
    assert!(is_relative("./x.yaml"));
    assert!(!is_relative("D:/x"));
}
