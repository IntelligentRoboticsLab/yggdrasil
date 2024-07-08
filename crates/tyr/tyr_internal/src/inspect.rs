pub trait Inspect {
    fn to_json(&self) -> String;
    fn update_from_json(&mut self, json: &str);
}
