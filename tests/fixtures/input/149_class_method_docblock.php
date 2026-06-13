<?php
class UserService {
    /**
     * @param int $id
     * @return User|null
     */
    public function findUser(int $id) { return User::findOne($id); }
    public function deleteUser(int $id): void { User::deleteAll(['id' => $id]); }
}
