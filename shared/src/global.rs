use uuid::Uuid;

pub fn uniq_id() -> String {
    String::from(&Uuid::new_v4().to_string()[..8])
}
