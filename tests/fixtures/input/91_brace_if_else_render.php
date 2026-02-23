<?php if ($currentArt) {
    echo $this->render('_addArtForm', [
        'product' => $model,
        'artsList' => $artsList,
        'currentArt' => $currentArt,
        'priceHelper' => $priceHelper,
]);
} else {
echo $this->render('_addCartForm', [
    'product' => $model,
    'artsList' => $artsList,
    'minPrice' => $minPrice,
    'maxPrice' => $maxPrice,
    'priceHelper' => $priceHelper,
]);
} ?>
