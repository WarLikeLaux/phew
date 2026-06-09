<?php

trait Sluggable
{
    protected string $slug = '';

    public function slug(): string
    {
        return $this->slug;
    }
}
?>
