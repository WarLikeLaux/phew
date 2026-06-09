use super::*;

#[test]
fn detects_class_block() {
    assert!(is_declaration_block(" final class Money { } "));
    assert!(is_declaration_block("interface Repo {}"));
    assert!(is_declaration_block("trait Sluggable {}"));
    assert!(is_declaration_block("enum Suit: string {}"));
}

#[test]
fn detects_named_function_block() {
    assert!(is_declaration_block("function helper($x) { return $x; }"));
}

#[test]
fn ignores_closures_and_statements() {
    assert!(!is_declaration_block("$cb = function ($x) { return $x; };"));
    assert!(!is_declaration_block("$this->title = 'Create';"));
    assert!(!is_declaration_block("foreach ($items as $item):"));
    assert!(!is_declaration_block("echo $value;"));
}

#[test]
fn detects_block_behind_docblock() {
    assert!(is_declaration_block("/** doc */ final class Money {}"));
}

#[test]
fn splits_class_brace_to_allman() {
    let normalized = "enum Suit: string {\ncase Hearts = 'h';\n}";
    let out = apply_psr12_declarations(normalized);
    assert_eq!(out, "enum Suit: string\n{\ncase Hearts = 'h';\n}");
}

#[test]
fn keeps_empty_method_body_inline() {
    let normalized = "final class Money {\npublic function __construct(int $a) {\n}\n}";
    let out = apply_psr12_declarations(normalized);
    assert_eq!(out, "final class Money\n{\npublic function __construct(int $a) {}\n}");
}

#[test]
fn inserts_blank_between_methods() {
    let normalized = "class C {\npublic function a() {\nreturn 1;\n}\npublic function b() {\nreturn 2;\n}\n}";
    let out = apply_psr12_declarations(normalized);
    assert_eq!(
        out,
        "class C\n{\npublic function a()\n{\nreturn 1;\n}\n\npublic function b()\n{\nreturn 2;\n}\n}"
    );
}

#[test]
fn no_blank_between_enum_cases() {
    let normalized = "enum E: int {\ncase A = 1;\ncase B = 2;\n}";
    let out = apply_psr12_declarations(normalized);
    assert_eq!(out, "enum E: int\n{\ncase A = 1;\ncase B = 2;\n}");
}

#[test]
fn allman_input_is_stable() {
    let allman = "enum Suit: string\n{\ncase Hearts = 'h';\n}";
    let out = apply_psr12_declarations(allman);
    assert_eq!(out, allman);
}

#[test]
fn control_flow_inside_method_stays_kr() {
    let normalized = "class C {\npublic function run() {\nif ($x) {\nreturn 1;\n}\n}\n}";
    let out = apply_psr12_declarations(normalized);
    assert_eq!(
        out,
        "class C\n{\npublic function run()\n{\nif ($x) {\nreturn 1;\n}\n}\n}"
    );
}
