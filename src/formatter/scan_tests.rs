use super::*;
use pretty_assertions::assert_eq;

#[test]
fn fat_arrow_returns_byte_offset_with_cyrillic() {
    let code = "'абв' => 'значение'";
    let pos = find_top_level_fat_arrow(code).unwrap();
    assert_eq!(&code[pos..pos + 2], "=>");
}

#[test]
fn assignment_equal_returns_byte_offset_with_cyrillic() {
    let code = "$абв = ['x' => 'y']";
    let pos = find_top_level_assignment_equal(code).unwrap();
    assert_eq!(&code[pos..=pos], "=");
    let lhs = code[..=pos].trim_end();
    assert_eq!(lhs, "$абв =");
}

#[test]
fn byte_offset_matches_ascii_char_index() {
    let chars: Vec<char> = "abc=>d".chars().collect();
    assert_eq!(byte_offset(&chars, 3), 3);
}

#[test]
fn top_level_or_finds_each_occurrence() {
    let code = "$hasContent = !empty($item->number) || !empty($item->articleCode) || !empty($item->name)";
    let positions = find_top_level_binary_op(code, BinaryOp::Or);
    assert_eq!(positions.len(), 2, "positions = {positions:?}");
}

#[test]
fn or_inside_parens_is_not_top_level() {
    let code = "$valid = in_array($status, [1, 2, 3]) && ($user->isAdmin() || $b) && $model->isPublished()";
    let and_positions = find_top_level_binary_op(code, BinaryOp::And);
    assert_eq!(and_positions.len(), 2, "&& positions = {and_positions:?}");
    let or_positions = find_top_level_binary_op(code, BinaryOp::Or);
    assert_eq!(
        or_positions.len(),
        0,
        "|| inside parens must not count: {or_positions:?}"
    );
}

#[test]
fn operator_inside_string_is_ignored() {
    let code = "$sql = $select . ' WHERE a = 1 || b = 2' . $joinClause . ' ORDER BY x, y'";
    let concat_positions = find_top_level_binary_op(code, BinaryOp::Concat);
    assert_eq!(concat_positions.len(), 3, "concat positions = {concat_positions:?}");
    let or_positions = find_top_level_binary_op(code, BinaryOp::Or);
    assert_eq!(
        or_positions.len(),
        0,
        "|| inside string must not count: {or_positions:?}"
    );
}

#[test]
fn concat_ignores_decimal_and_spread() {
    let decimal = find_top_level_binary_op("$x = 1.5 + $rate", BinaryOp::Concat);
    assert_eq!(decimal.len(), 0, "decimal point must not count: {decimal:?}");
    let spread = find_top_level_binary_op("$x = [...$a, ...$b]", BinaryOp::Concat);
    assert_eq!(spread.len(), 0, "spread dots must not count: {spread:?}");
    let real = find_top_level_binary_op("$x = $a . $b", BinaryOp::Concat);
    assert_eq!(real.len(), 1, "real concat must count: {real:?}");
}
