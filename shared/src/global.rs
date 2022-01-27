use uuid::Uuid;

pub fn uniq_id() -> Uuid {
    Uuid::new_v4()
}
