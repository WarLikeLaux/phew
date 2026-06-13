<?php
enum Status: string {
    case Active = 'active';
    case Blocked = 'blocked';
    public function label(): string { return ucfirst($this->value); }
}
