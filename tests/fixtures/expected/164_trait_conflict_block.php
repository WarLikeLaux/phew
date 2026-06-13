<?php

class Greeter
{
    use Hello, World {
        Hello::say insteadof World;
        World::say as sayWorld;
    }
}
?>
