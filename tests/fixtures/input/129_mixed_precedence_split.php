<?php $allowed = $user->isActive() && $user->hasRole('editor') || $user->isAdmin() && $user->isOwner($model) || $user->isSuperUser(); ?>
