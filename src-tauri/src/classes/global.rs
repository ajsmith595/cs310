use uuid::Uuid;

pub fn uniq_id() -> String {
  Uuid::new_v4().to_string()
}
