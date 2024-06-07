// @generated automatically by Diesel CLI.

diesel::table! {
    anime_broadcast (id) {
        id -> Nullable<Integer>,
        mikan_id -> Integer,
        year -> Integer,
        season -> Integer,
    }
}

diesel::table! {
    anime_filter (id) {
        id -> Nullable<Integer>,
        mikan_id -> Integer,
        filter_type -> Text,
        filter_val -> Integer,
        object -> Integer,
    }
}

diesel::table! {
    anime_list (id) {
        id -> Nullable<Integer>,
        mikan_id -> Integer,
        anime_name -> Text,
        update_day -> Integer,
        img_url -> Text,
        anime_type -> Integer,
        subscribe_status -> Integer,
        bangumi_id -> Integer,
        bangumi_rank -> Text,
        bangumi_summary -> Text,
        website -> Text,
    }
}

diesel::table! {
    anime_seed (id) {
        id -> Nullable<Integer>,
        mikan_id -> Integer,
        subgroup_id -> Integer,
        episode -> Integer,
        seed_name -> Text,
        seed_url -> Text,
        seed_status -> Integer,
        seed_size -> Text,
    }
}

diesel::table! {
    anime_subgroup (id) {
        id -> Nullable<Integer>,
        subgroup_id -> Integer,
        subgroup_name -> Text,
    }
}

diesel::table! {
    anime_task (id) {
        id -> Nullable<Integer>,
        mikan_id -> Integer,
        episode -> Integer,
        torrent_name -> Text,
        qb_task_status -> Integer,
        rename_status -> Integer,
        filename -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    anime_broadcast,
    anime_filter,
    anime_list,
    anime_seed,
    anime_subgroup,
    anime_task,
);
