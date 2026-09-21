use super::*;

#[test]
fn raw_parameter_accepts_every_typed_kind_as_one_argument() {
    let fixtures = [
        (
            RawParameterKind::Recipe,
            "busybox",
            RawParameterValue::Recipe("busybox".into()),
        ),
        (
            RawParameterKind::Image,
            "core-image-minimal",
            RawParameterValue::Image("core-image-minimal".into()),
        ),
        (
            RawParameterKind::Target,
            "virtual/kernel",
            RawParameterValue::Target("virtual/kernel".into()),
        ),
        (
            RawParameterKind::Task,
            "do_compile",
            RawParameterValue::Task("do_compile".into()),
        ),
        (
            RawParameterKind::UserInterface,
            "knotty",
            RawParameterValue::UserInterface("knotty".into()),
        ),
        (
            RawParameterKind::File,
            "/tmp/Raw Mode/recipe.bb",
            RawParameterValue::File("/tmp/Raw Mode/recipe.bb".into()),
        ),
        (
            RawParameterKind::Integer,
            "4294967295",
            RawParameterValue::Integer(MAX_RAW_INTEGER),
        ),
        (
            RawParameterKind::Text,
            "例え:値/=+",
            RawParameterValue::Text("例え:値/=+".into()),
        ),
        (
            RawParameterKind::Multiconfig,
            "lib32",
            RawParameterValue::Multiconfig("lib32".into()),
        ),
    ];

    for (kind, input, expected) in fixtures {
        let value = parsed(kind, input);
        assert_eq!(value, expected);
        assert_eq!(value.argument(), input);
    }
}
