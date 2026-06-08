use proptest::prelude::*;

use phew::formatter::Formatter;
use phew::parser::ast::{self, Node};
use phew::parser::lexer;

fn pipeline(source: &str) -> String {
    Formatter::default().format_source(source)
}

fn ast_of(source: &str) -> Vec<Node> {
    ast::parse(lexer::tokenize(source))
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, PartialEq)]
enum Skeleton {
    Tag(String, Vec<Skeleton>),
    Text(String),
}

fn skeleton(nodes: &[Node]) -> Vec<Skeleton> {
    nodes
        .iter()
        .filter_map(|node| match node {
            Node::Element { name, children, .. } => Some(Skeleton::Tag(name.to_lowercase(), skeleton(children))),
            Node::Text(value) => {
                let collapsed = collapse_whitespace(value);
                if collapsed.is_empty() {
                    None
                } else {
                    Some(Skeleton::Text(collapsed))
                }
            }
            Node::PhpBlock(_) | Node::PhpEcho(_) | Node::Doctype(_) | Node::Comment(_) => None,
        })
        .collect()
}

fn php_expressions() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "$model->id",
        "$user->name",
        "$a + $b",
        "count($items)",
        "Html::encode($value)",
        "'literal'",
        "42",
        "$flag ? $yes : $no",
    ])
}

fn html_tags() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "div", "section", "article", "p", "span", "ul", "li", "a", "strong", "em", "td", "tr",
    ])
}

fn html_attributes() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            " class=\"box\"",
            " id=\"main\"",
            " data-role=\"item\"",
            " href=\"#\"",
            " type=\"text\"",
        ]),
        0..3,
    )
    .prop_map(|parts| parts.concat())
}

fn leaf_fragments() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec!["Lorem ipsum", "Привет мир", "one two three", "single", "x  y"])
            .prop_map(str::to_owned),
        php_expressions().prop_map(|expr| format!("<?= {expr} ?>")),
        php_expressions().prop_map(|expr| format!("<?php echo {expr}; ?>")),
        prop::sample::select(vec!["$x = 1;", "$total += $n;", "$name = $model->name;"])
            .prop_map(|stmt| format!("<?php {stmt} ?>")),
        Just("<br>".to_owned()),
        Just("<hr>".to_owned()),
        Just("<img src=\"pic.png\" alt=\"image\">".to_owned()),
        Just("<input type=\"text\" name=\"field\">".to_owned()),
        Just("<!-- a note -->".to_owned()),
    ]
}

fn view_fragments() -> impl Strategy<Value = String> {
    leaf_fragments().prop_recursive(4, 48, 4, |inner| {
        prop_oneof![
            (
                html_tags(),
                html_attributes(),
                prop::collection::vec(inner.clone(), 0..4)
            )
                .prop_map(|(tag, attrs, children)| format!("<{tag}{attrs}>{}</{tag}>", children.concat())),
            (php_expressions(), prop::collection::vec(inner.clone(), 1..4))
                .prop_map(|(cond, children)| format!("<?php if ({cond}): ?>{}<?php endif; ?>", children.concat())),
            prop::collection::vec(inner, 1..4).prop_map(|children| format!(
                "<?php foreach ($items as $item): ?>{}<?php endforeach; ?>",
                children.concat()
            )),
        ]
    })
}

fn view_documents() -> impl Strategy<Value = String> {
    prop::collection::vec(view_fragments(), 0..5).prop_map(|parts| parts.join("\n"))
}

fn structure_documents() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        prop::sample::select(vec!["alpha", "beta gamma", "Текст", "single"]).prop_map(str::to_owned),
        Just("<br>".to_owned()),
        Just("<hr>".to_owned()),
    ];
    let tags = prop::sample::select(vec!["div", "section", "p", "span", "ul", "li", "strong"]);
    let fragment = leaf.prop_recursive(4, 32, 3, move |inner| {
        (tags.clone(), prop::collection::vec(inner, 0..4))
            .prop_map(|(tag, children)| format!("<{tag}>{}</{tag}>", children.concat()))
    });
    prop::collection::vec(fragment, 0..4).prop_map(|parts| parts.concat())
}

fn arbitrary_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..200).prop_map(|chars| chars.into_iter().collect())
}

fn html_token_noise() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            "<", ">", "<?php", "?>", "<?=", "<div>", "</div>", "<span>", "</span>", "\"", "'", "$x", "echo ", ";",
            "\n", "  ", "<!--", "-->", "</", "/>", "{", "}", "(", ")", "if (", "):", "endif;", "foreach", "text",
        ]),
        0..48,
    )
    .prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn formatting_is_idempotent_on_views(source in view_documents()) {
        let once = pipeline(&source);
        let twice = pipeline(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn output_reparses_to_a_stable_ast(source in view_documents()) {
        let once = pipeline(&source);
        let ast_once = ast_of(&once);
        let reformatted = Formatter::default().format(&ast_once);
        let ast_twice = ast_of(&reformatted);
        prop_assert_eq!(ast_once, ast_twice);
    }

    #[test]
    fn formatting_preserves_html_structure(source in structure_documents()) {
        let before = skeleton(&ast_of(&source));
        let after = skeleton(&ast_of(&pipeline(&source)));
        prop_assert_eq!(before, after);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    #[test]
    fn never_panics_on_arbitrary_text(source in arbitrary_text()) {
        let once = pipeline(&source);
        let _ = pipeline(&once);
    }

    #[test]
    fn never_panics_on_html_token_noise(source in html_token_noise()) {
        let once = pipeline(&source);
        let _ = pipeline(&once);
    }
}

#[test]
fn every_expected_fixture_is_a_fixpoint() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/expected");
    let formatter = Formatter::default();
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no expected fixtures found in {}", dir.display());
    for path in paths {
        let expected = std::fs::read_to_string(&path).unwrap();
        let formatted = formatter.format_source(&expected);
        assert_eq!(formatted, expected, "fixture {} is not a fixpoint", path.display());
    }
}
