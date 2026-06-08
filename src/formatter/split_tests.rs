
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
