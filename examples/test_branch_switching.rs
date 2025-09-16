use ai_c::{
    config::GitConfig,
    git::GitService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GitConfig::default();
    let git_service = GitService::new(&config).await?;

    println!("=== Testing Branch Switching Functionality ===");

    // List current branches
    println!("\n1. Listing all branches:");
    match git_service.list_branches() {
        Ok(branches) => {
            for branch in branches {
                let status = if branch.is_current { " (current)" } else { "" };
                println!("  - {}{}", branch.name, status);
            }
        }
        Err(e) => {
            println!("Error listing branches: {}", e);
        }
    }

    // Try switching to feature/branch-switching if it exists
    println!("\n2. Attempting to switch to 'feature/branch-switching':");
    match git_service.switch_branch("feature/branch-switching").await {
        Ok(_) => {
            println!("✅ Successfully switched to 'feature/branch-switching'");

            // List branches again to confirm switch
            println!("\n3. Listing branches after switch:");
            match git_service.list_branches() {
                Ok(branches) => {
                    for branch in branches {
                        let status = if branch.is_current { " (current)" } else { "" };
                        println!("  - {}{}", branch.name, status);
                    }
                }
                Err(e) => {
                    println!("Error listing branches: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to switch branch: {}", e);
        }
    }

    // Switch back to master
    println!("\n4. Switching back to 'master':");
    match git_service.switch_branch("master").await {
        Ok(_) => {
            println!("✅ Successfully switched back to 'master'");
        }
        Err(e) => {
            println!("❌ Failed to switch back to master: {}", e);
        }
    }

    println!("\n=== Branch switching test completed ===");
    Ok(())
}