use super::has_placeholder;

#[test]
fn placeholders() {
    let templated = [
        "{{cell}}",
        "etcd-{{ .Values.cell }}",
        "${NAME}",
        "$(NAME)",
        "$NAME",
        "process-item-$item",
        "a-$_x",
        "é-$X",
    ];
    for value in templated {
        assert!(has_placeholder(value), "{value}");
    }
    for value in [
        "", "web", "price-$5", "a$", "$-x", "$é", "naïve", "{team}", "a}}b{",
    ] {
        assert!(!has_placeholder(value), "{value}");
    }
}
