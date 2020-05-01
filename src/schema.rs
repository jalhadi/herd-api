table! {
    component_events (id) {
        id -> Int4,
        component_id -> Varchar,
        device_id -> Varchar,
        data -> Jsonb,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        event_created_at -> Timestamp,
    }
}

table! {
    components (id) {
        id -> Varchar,
        module_type_id -> Varchar,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        description -> Nullable<Varchar>,
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
    modules (id) {
        id -> Varchar,
        device_type_id -> Varchar,
        name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        parent_id -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
    }
}

joinable!(component_events -> components (component_id));
joinable!(component_events -> devices (device_id));
joinable!(components -> modules (module_type_id));
joinable!(devices -> device_types (device_type_id));
joinable!(modules -> device_types (device_type_id));

allow_tables_to_appear_in_same_query!(
    component_events,
    components,
    device_types,
    devices,
    modules,
);
