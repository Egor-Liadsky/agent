use anyhow::Result;
use rmcp::{RoleClient, ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::json;
use std::{process::Stdio, time::Duration};
use tokio::process::Command;

async fn call(
    service: &rmcp::service::RunningService<RoleClient, ()>,
    name: &str,
    id: &str,
) -> Result<rmcp::model::CallToolResult> {
    let args = json!({"id":id}).as_object().cloned().unwrap();
    Ok(service
        .peer()
        .call_tool(CallToolRequestParams::new(name.to_string()).with_arguments(args))
        .await?)
}

#[tokio::test]
async fn real_server_reads_latest_ticket_and_fails_closed_for_unknown_id() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("crm.json");
    std::fs::write(
        &file,
        serde_json::to_vec(&json!({
            "version": 1,
            "users": [{"id":"U-001","name":"Демо Альфа","support_notes":"только профиль альфа"}],
            "tickets": [{"id":"T-001","user_id":"U-001","subject":"Первый вопрос","description":"первое описание","status":"open","created_at":"now"}]
        }))?,
    )?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_crm-mcp"));
    command.arg("--data").arg(&file).kill_on_drop(true);
    let (transport, _) = TokioChildProcess::builder(command)
        .stderr(Stdio::null())
        .spawn()?;
    let service = tokio::time::timeout(Duration::from_secs(10), ().serve(transport)).await??;
    let first = call(&service, "crm_get_ticket", "T-001").await?;
    assert_eq!(
        first.structured_content.as_ref().unwrap()["description"],
        "первое описание"
    );

    std::fs::write(
        &file,
        serde_json::to_vec(&json!({
            "version": 1,
            "users": [{"id":"U-001","name":"Демо Альфа","support_notes":"только профиль альфа"}],
            "tickets": [{"id":"T-001","user_id":"U-001","subject":"Обновлённый вопрос","description":"обновлённое описание","status":"open","created_at":"now"}]
        }))?,
    )?;
    let updated = call(&service, "crm_get_ticket", "T-001").await?;
    assert_eq!(
        updated.structured_content.as_ref().unwrap()["description"],
        "обновлённое описание"
    );
    let missing = call(&service, "crm_get_ticket", "T-404").await;
    match missing {
        Ok(result) => assert_eq!(result.is_error, Some(true)),
        Err(error) => assert!(error.to_string().contains("T-404")),
    }
    let _ = service.cancel().await;
    Ok(())
}
