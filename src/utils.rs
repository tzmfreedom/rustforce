pub fn substring_before(body: &str, separator: &str) -> String {
  match body.find(separator) {
    Some(i) => body.get(..i).unwrap().to_string(),
    None => body.to_string(),
  }
}
