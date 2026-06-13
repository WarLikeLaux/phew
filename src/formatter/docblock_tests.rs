use super::*;

fn assert_expand_cases(cases: &[(&str, Option<&str>)]) {
    for (input, expected) in cases {
        let expected = expected.map(ToString::to_string);
        assert_eq!(expand_single_line_docblock(input), expected, "input: {input}");
    }
}

fn assert_extract_cases(cases: &[(&str, Option<&str>)]) {
    for (input, expected) in cases {
        let expected = expected.map(ToString::to_string);
        assert_eq!(extract_docblock_body(input), expected, "input: {input}");
    }
}

fn assert_docblock_only_cases(cases: &[(&str, bool)]) {
    for (input, expected) in cases {
        assert_eq!(is_docblock_only(input), *expected, "input: {input}");
    }
}

fn assert_var_normalize_cases(cases: &[(&str, &str)]) {
    for (input, expected) in cases {
        assert_eq!(normalize_var_body(input), *expected, "input: {input}");
    }
}

#[test]
fn expand_docblock_cases() {
    assert_expand_cases(&[
        ("/** @var string $x */", Some("/**\n * @var string $x\n */")),
        ("/** */", Some("/**\n */")),
    ]);
}

#[test]
fn extract_docblock_cases() {
    assert_extract_cases(&[
        ("/** @var string $x */", Some("@var string $x")),
        ("/** */", None),
        ("/** @var $this yii\\web\\View */", Some("@var yii\\web\\View $this")),
    ]);
}

#[test]
fn merge_docblock_cases() {
    assert_eq!(
        merge_docblock_bodies(&["@var string $x".to_string()]),
        "/**\n * @var string $x\n */"
    );

    let bodies = vec!["Description".to_string(), String::new(), "@var string $x".to_string()];
    assert_eq!(
        merge_docblock_bodies(&bodies),
        "/**\n * Description\n *\n * @var string $x\n */"
    );
}

#[test]
fn descriptions_and_vars_with_separator() {
    let descs = vec!["Hello".to_string()];
    let vars = vec!["@var int $x".to_string()];
    let result = merge_descriptions_and_vars(&descs, &vars);
    assert_eq!(result, vec!["Hello", "", "@var int $x"]);
}

#[test]
fn docblock_only_cases() {
    assert_docblock_only_cases(&[
        ("/** @var string $x */", true),
        ("/**\n * @var string $x\n */", true),
        ("$x = 1;", false),
    ]);
}

#[test]
fn emit_docblock_php_normalizes_existing_indent() {
    let mut output = String::new();
    emit_docblock_php(
        "        /**\n         * @var Foo $foo\n         */",
        "    ",
        &mut output,
    );
    assert_eq!(output, "    <?php\n    /**\n     * @var Foo $foo\n     */\n    ?>\n");
}

#[test]
fn normalize_var_cases() {
    assert_var_normalize_cases(&[
        ("@var $model User", "@var User $model"),
        ("@var User $model", "@var User $model"),
        ("@return string", "@return string"),
    ]);
}
