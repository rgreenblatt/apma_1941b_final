table! {
    contributions (id) {
        id -> Int4,
        repo_id -> Int4,
        user_id -> Int4,
        num -> Int4,
    }
}

table! {
    dependencies (id) {
        id -> Int4,
        repo_from_id -> Int4,
        repo_to_id -> Int4,
    }
}

table! {
    repos (id) {
        id -> Int4,
        owner_name -> Varchar,
    }
}

table! {
    users (id) {
        id -> Int4,
        login -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    contributions,
    dependencies,
    repos,
    users,
);