<?php for ($i = 0; $i < 3; $i++) { ?>
    <div class="item">
        <span><?= $i ?></span>
    </div>
<?php } ?>
<?php while ($row = $stmt->fetch()) { ?>
    <tr>
        <td><?= $row['name'] ?></td>
    </tr>
<?php } ?>
