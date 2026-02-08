const PHP_KEYWORDS: &[&str] = &[
    "if", "elseif", "else", "foreach", "for", "while", "switch", "catch", "match",
];

pub fn format_php_code(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if ch == '\'' || ch == '"' {
            i = skip_string_literal(&chars, i, &mut result);
            continue;
        }

        if ch == '=' && i + 1 < len && chars[i + 1] == '>' {
            i = format_fat_arrow(&chars, i, &mut result);
            continue;
        }

        if ch == ',' {
            i = format_comma(&chars, i, &mut result);
            continue;
        }

        if ch.is_alphabetic() {
            i = format_keyword(&chars, i, &mut result);
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

pub fn join_php_lines(code: &str) -> String {
    code.lines().map(|line| line.trim()).collect::<Vec<_>>().join(" ")
}

pub fn split_by_chain(code: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\'' || chars[i] == '"' {
            let quote = chars[i];
            current.push(quote);
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    current.push(chars[i]);
                    i += 1;
                    if i < len {
                        current.push(chars[i]);
                        i += 1;
                    }
                    continue;
                }
                current.push(chars[i]);
                i += 1;
            }
            if i < len {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        if chars[i] == '-' && i + 1 < len && chars[i + 1] == '>' {
            parts.push(current.trim_end().to_string());
            current = String::from("->");
            i += 2;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        parts.push(current.trim_end().to_string());
    }

    parts
}

fn skip_string_literal(chars: &[char], start: usize, result: &mut String) -> usize {
    let quote = chars[start];
    let len = chars.len();
    result.push(quote);
    let mut i = start + 1;

    while i < len && chars[i] != quote {
        if chars[i] == '\\' {
            result.push(chars[i]);
            i += 1;
            if i < len {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }

    if i < len {
        result.push(chars[i]);
        i += 1;
    }

    i
}

fn format_fat_arrow(chars: &[char], start: usize, result: &mut String) -> usize {
    if !result.ends_with(' ') {
        result.push(' ');
    }
    result.push_str("=>");
    let i = start + 2;
    if i < chars.len() && chars[i] != ' ' {
        result.push(' ');
    }
    i
}

fn format_comma(chars: &[char], start: usize, result: &mut String) -> usize {
    result.push(',');
    let i = start + 1;
    if i < chars.len() && chars[i] != ' ' && chars[i] != '\n' {
        result.push(' ');
    }
    i
}

fn format_keyword(chars: &[char], start: usize, result: &mut String) -> usize {
    let len = chars.len();
    let mut i = start;

    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    let word: String = chars[start..i].iter().collect();

    if PHP_KEYWORDS.contains(&word.as_str()) && i < len && chars[i] == '(' {
        result.push_str(&word);
        result.push(' ');
    } else {
        result.push_str(&word);
    }

    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn keyword_space() {
        assert_eq!(format_php_code("if($x):"), "if ($x):");
    }

    #[test]
    fn keyword_already_spaced() {
        assert_eq!(format_php_code("if ($x):"), "if ($x):");
    }

    #[test]
    fn foreach_keyword() {
        assert_eq!(
            format_php_code("foreach($items as $item):"),
            "foreach ($items as $item):"
        );
    }

    #[test]
    fn arrow_spacing() {
        assert_eq!(format_php_code("'id'=>$item->id"), "'id' => $item->id");
    }

    #[test]
    fn arrow_already_spaced() {
        assert_eq!(format_php_code("'id' => $item->id"), "'id' => $item->id");
    }

    #[test]
    fn comma_spacing() {
        assert_eq!(format_php_code("$a,$b,$c"), "$a, $b, $c");
    }

    #[test]
    fn comma_already_spaced() {
        assert_eq!(format_php_code("$a, $b, $c"), "$a, $b, $c");
    }

    #[test]
    fn object_arrow_untouched() {
        assert_eq!(format_php_code("$model->title"), "$model->title");
    }

    #[test]
    fn string_content_untouched() {
        assert_eq!(
            format_php_code("Html::a('foo=>bar','baz')"),
            "Html::a('foo=>bar', 'baz')"
        );
    }

    #[test]
    fn complex_yii_call() {
        let input = "Html::a($item->name,['item/view','id'=>$item->id],['class'=>'btn btn-primary'])";
        let expected = "Html::a($item->name, ['item/view', 'id' => $item->id], ['class' => 'btn btn-primary'])";
        assert_eq!(format_php_code(input), expected);
    }

    #[test]
    fn endif_unchanged() {
        assert_eq!(format_php_code("endif;"), "endif;");
    }

    #[test]
    fn echo_expression() {
        assert_eq!(
            format_php_code("Html::encode($model->title)"),
            "Html::encode($model->title)"
        );
    }
}
