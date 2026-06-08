
use super::*;
use pretty_assertions::assert_eq;

fn assert_format_cases(cases: &[(&str, &str)]) {
    for (input, expected) in cases {
        assert_eq!(format_php_code(input), *expected, "input: {input}");
    }
}

#[test]
fn spacing_and_unchanged_cases() {
    assert_format_cases(&[
        ("if($x):", "if ($x):"),
        ("if ($x):", "if ($x):"),
        ("foreach($items as $item):", "foreach ($items as $item):"),
        ("'id'=>$item->id", "'id' => $item->id"),
        ("'id' => $item->id", "'id' => $item->id"),
        ("$a,$b,$c", "$a, $b, $c"),
        ("$a, $b, $c", "$a, $b, $c"),
        ("$model->title", "$model->title"),
        ("Html::a('foo=>bar','baz')", "Html::a('foo=>bar', 'baz')"),
        ("endif;", "endif;"),
        ("Html::encode($model->title)", "Html::encode($model->title)"),
        ("$name='World';", "$name = 'World';"),
        ("declare(strict_types=1);", "declare(strict_types=1);"),
        ("$a==$b", "$a==$b"),
        ("$a!=$b", "$a!=$b"),
        ("$a>=$b", "$a>=$b"),
        ("$a<=$b", "$a<=$b"),
        ("$a??=$b", "$a??=$b"),
    ]);
}

#[test]
fn complex_yii_call() {
    assert_eq!(
        format_php_code("Html::a($item->name,['item/view','id'=>$item->id],['class'=>'btn btn-primary'])"),
        "Html::a($item->name, ['item/view', 'id' => $item->id], ['class' => 'btn btn-primary'])"
    );
}

#[test]
fn split_concat_expression() {
    let code = "'a' . $b . 'c'";
    assert_eq!(
        split_by_concat(code),
        vec!["'a'".to_string(), "$b".to_string(), "'c'".to_string()]
    );
}
