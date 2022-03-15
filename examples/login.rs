use rustforce::{Client, Error};
// use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
  let mut client = Client::new("".to_string(), "".to_string());
  client
    .login_by_soap(
      "team@xe432.elastify.eu".to_string(),
      "hchkyMTv2f2qmt2euEVqX9dxG2RvLaeMdRveXP2Cb3kBWKDsliCkVhk3RvTeQZDPnCmhyHu".to_string(),
    )
    .await?;
  Ok(())
}
