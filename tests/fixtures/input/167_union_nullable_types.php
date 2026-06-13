<?php
function findOwner(int|string $id, ?User $fallback = null): User|null {
return User::findOne($id) ?? $fallback;
}
