-- Your SQL goes here
-- ----------------------------
-- Table structure for anime_broadcast
-- ----------------------------
DROP TABLE IF EXISTS "anime_broadcast";
CREATE TABLE anime_broadcast (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mikan_id INTEGER NOT NULL,
  year INTEGER NOT NULL,
  season INTEGER NOT NULL
);

-- ----------------------------
-- Table structure for anime_filter
-- ----------------------------
DROP TABLE IF EXISTS "anime_filter";
CREATE TABLE "anime_filter" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT,
  "mikan_id" INTEGER NOT NULL,
  "filter_type" TEXT NOT NULL,
  "filter_val" INTEGER NOT NULL,
  "object" INTEGER NOT NULL DEFAULT 0
);

-- ----------------------------
-- Table structure for anime_list
-- ----------------------------
DROP TABLE IF EXISTS "anime_list";
CREATE TABLE "anime_list" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT,
  "mikan_id" INTEGER NOT NULL DEFAULT NULL,
  "anime_name" TEXT NOT NULL DEFAULT NULL,
  "update_day" INTEGER NOT NULL DEFAULT NULL,
  "img_url" TEXT NOT NULL,
  "anime_type" INTEGER NOT NULL DEFAULT NULL,
  "subscribe_status" INTEGER NOT NULL DEFAULT NULL
);

-- ----------------------------
-- Table structure for anime_seed
-- ----------------------------
DROP TABLE IF EXISTS "anime_seed";
CREATE TABLE "anime_seed" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT,
  "mikan_id" INTEGER NOT NULL,
  "subgroup_id" INTEGER NOT NULL,
  "episode" INTEGER NOT NULL,
  "seed_name" TEXT NOT NULL,
  "seed_url" TEXT NOT NULL,
  "seed_status" INTEGER NOT NULL DEFAULT NULL,
  "seed_size" TEXT NOT NULL,
  UNIQUE ("seed_url" ASC)
);

-- ----------------------------
-- Table structure for anime_subgroup
-- ----------------------------
DROP TABLE IF EXISTS "anime_subgroup";
CREATE TABLE anime_subgroup (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  subgroup_id INTEGER NOT NULL,
  subgroup_name TEXT NOT NULL,
  UNIQUE(subgroup_id)
);

-- ----------------------------
-- Table structure for anime_task
-- ----------------------------
DROP TABLE IF EXISTS "anime_task";
CREATE TABLE "anime_task" (
  "id" INTEGER PRIMARY KEY AUTOINCREMENT,
  "mikan_id" INTEGER NOT NULL,
  "episode" INTEGER NOT NULL,
  "torrent_name" TEXT NOT NULL,
  "qb_task_status" INTEGER NOT NULL DEFAULT NULL,
  "rename_status" INTEGER NOT NULL,
  "filename" TEXT NOT NULL
);