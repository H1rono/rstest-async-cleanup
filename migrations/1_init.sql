-- key-value store

CREATE TABLE IF NOT EXISTS `entries`
(
    `id`         BINARY(16) NOT NULL PRIMARY KEY,
    `key`        TEXT,
    `value`      TEXT,
    `created_at` DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at` DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `deleted_at` DATETIME            DEFAULT NULL
);
