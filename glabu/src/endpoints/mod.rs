pub mod packages;
pub mod profiles;
pub mod projects;
pub mod releases;
pub mod setup;

use serde::Serialize;
#[derive(Serialize)]
pub struct PrintOutput<T>
where
    T: Serialize,
{
    pub status: String,
    pub output: T,
}
