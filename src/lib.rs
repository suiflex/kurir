pub mod error;
pub mod harness;
pub mod hooks;
pub mod jsonc;
pub mod model;
pub mod registration;
pub mod skills;
pub mod update;

pub use error::Error;
pub use harness::Harness;
pub use hooks::{HookSpec, add_hook, hook_file, hook_path, is_hook_present, register_hook};
pub use model::{RegistrationOptions, RegistrationResult, Scope, ServerSpec, Transport};
pub use registration::{DoctorReport, doctor, register};
pub use skills::{install_skill, skills_dir, target_skills_dir};
pub use update::run as update;
