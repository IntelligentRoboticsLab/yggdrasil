pub trait Inspect {
    fn name(&self) -> &'static str;
    fn to_json(&self) -> String;
    fn update_from_json(&mut self, json: &str);
}
