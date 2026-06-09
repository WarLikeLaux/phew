<?php $valid = in_array($status, [1, 2, 3]) && ($user->isAdmin() || $user->isModerator() || $user->isOwner()) && $model->isPublished(); ?>
