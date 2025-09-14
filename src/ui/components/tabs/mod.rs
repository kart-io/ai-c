//! Tab components
//!
//! Individual tab implementations for the six main tabs.

pub mod branches;
pub mod gitflow;
pub mod remotes;
pub mod stash;
pub mod status;
pub mod tags;

pub use branches::BranchesTabComponent;
pub use gitflow::GitFlowTabComponent;
pub use remotes::RemotesTabComponent;
pub use stash::StashTabComponent;
pub use status::StatusTabComponent;
pub use tags::TagsTabComponent;
