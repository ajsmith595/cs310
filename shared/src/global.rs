use uuid::Uuid;
/**
 * Utility function for quickly getting a Uuid
 */
pub fn uniq_id() -> Uuid {
    Uuid::new_v4()
}
