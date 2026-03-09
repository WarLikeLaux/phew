<?php
$query = $model->find()
->where(['status' => 1])
->andWhere(['>', 'price', 0])
->orderBy(['created_at' => SORT_DESC])
->limit(10)
->all();

$result = $builder->setName($name)
->setType($type)
->build();
?>
