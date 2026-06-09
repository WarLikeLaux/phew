<?php match ($model->status) { 'active' => $model->activate(), 'blocked' => $model->block(), 'pending' => $model->markPending(), default => $model->reset() }; ?>
