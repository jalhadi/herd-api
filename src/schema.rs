table! {
    accounts (id) {
        id -> Varchar,
        secret_key -> Varchar,
        cipher_iv -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        max_requests_per_minute -> Int4,
        max_connections -> Int4,
    }
}

table! {
    device_types (id) {
        id -> Varchar,
        account_id -> Varchar,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        description -> Nullable<Varchar>,
    }
}

table! {
    devices (id) {
        id -> Varchar,
        device_type_id -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    logs (id) {
        id -> Int4,
        account_id -> Varchar,
        level -> Varchar,
        data -> Nullable<Jsonb>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    topics (id) {
        id -> Varchar,
        account_id -> Varchar,
        name -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    webhook_topics (id) {
        id -> Int4,
        webhook_id -> Int4,
        topic_id -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    webhooks (id) {
        id -> Int4,
        account_id -> Varchar,
        url -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(devices -> device_types (device_type_id));
joinable!(webhook_topics -> topics (topic_id));
joinable!(webhook_topics -> webhooks (webhook_id));

allow_tables_to_appear_in_same_query!(
    accounts,
    device_types,
    devices,
    logs,
    topics,
    webhook_topics,
    webhooks,
);
