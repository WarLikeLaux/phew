<?php

use yii\base\Model;

#[\Attribute]
class FilterForm extends Model
{
    #[Required]
    public string $query = '';

    public function rules()
    {
        return [['query', 'string']];
    }
}
?>
