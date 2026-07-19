use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Response {
    #[serde(flatten)]
    pub cmd: Commands,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd", content = "data")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum Commands {
    Authorize { code: String },
    Error { code: u32, message: String },
}
