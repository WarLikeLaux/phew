<?php
#[Route('/users/{id}', methods: ['GET'])]
#[Cache(ttl: 3600)]
function show(int $id): Response {
return render($id);
}
