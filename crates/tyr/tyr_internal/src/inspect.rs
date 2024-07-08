/// Like a structured `Debug` that can be modified from JSON.
///
/// To derive this trait, use `[derive(Serialize, Deserialize, Inspect)]`. You shouldn't need to
/// manually derive this trait.
pub trait Inspect {
    /// Name of the resource fit for display.
    fn name(&self) -> &'static str;
    /// Serialize the resource to JSON.
    fn to_json(&self) -> String;
    /// Deserialize the resource from JSON and update it in place.
    fn update_from_json(&mut self, json: &str);
}
