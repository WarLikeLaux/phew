<?php $sql = $select . ' FROM users WHERE status = 1 || status = 2 || archived = 0' . $joinClause . ' ORDER BY created_at DESC, id ASC'; ?>
