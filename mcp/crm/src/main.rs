use anyhow::{Context, Result};
use clap::Parser;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::JsonSchema;
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: String,
    name: String,
    #[serde(default)]
    support_notes: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Ticket {
    id: String,
    user_id: String,
    subject: String,
    description: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Database {
    version: u32,
    #[serde(default)]
    users: Vec<User>,
    #[serde(default)]
    tickets: Vec<Ticket>,
}

#[derive(Debug, Parser)]
#[command(about = "MCP-сервер только для чтения JSON-файла мини-CRM")]
struct Args {
    #[arg(long)]
    data: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct IdArgs {
    id: String,
}

#[derive(Clone)]
struct CrmServer {
    path: PathBuf,
    router: ToolRouter<Self>,
}

#[tool_router]
impl CrmServer {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            router: Self::tool_router(),
        }
    }

    fn read(&self) -> Result<Database, String> {
        let raw = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("CRM-файл {} недоступен: {e}", self.path.display()))?;
        let database: Database =
            serde_json::from_str(&raw).map_err(|e| format!("ошибка формата CRM JSON: {e}"))?;
        if database.version != 1 {
            return Err(format!(
                "неподдерживаемая версия CRM JSON: {}",
                database.version
            ));
        }
        Ok(database)
    }

    #[tool(description = "Прочитать один тикет поддержки по ID из файла CRM")]
    fn crm_get_ticket(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, String> {
        let db = self.read()?;
        let ticket = db
            .tickets
            .iter()
            .find(|item| item.id == args.id)
            .ok_or_else(|| format!("тикет {} не найден", args.id))?;
        Ok(CallToolResult::structured(
            serde_json::to_value(ticket).map_err(|e| e.to_string())?,
        ))
    }

    #[tool(description = "Прочитать все тикеты поддержки из файла CRM")]
    fn crm_list_tickets(&self) -> Result<CallToolResult, String> {
        let db = self.read()?;
        Ok(CallToolResult::structured(serde_json::json!({
            "tickets": db.tickets,
        })))
    }

    #[tool(description = "Прочитать одного пользователя CRM по ID")]
    fn crm_get_user(&self, Parameters(args): Parameters<IdArgs>) -> Result<CallToolResult, String> {
        let db = self.read()?;
        let user = db
            .users
            .iter()
            .find(|item| item.id == args.id)
            .ok_or_else(|| format!("пользователь {} не найден", args.id))?;
        Ok(CallToolResult::structured(
            serde_json::to_value(user).map_err(|e| e.to_string())?,
        ))
    }
}

#[tool_handler(router = self.router)]
impl ServerHandler for CrmServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(format!(
                "Только чтение CRM JSON из файла {}",
                self.path.display()
            ))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    CrmServer::new(args.data)
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await
        .context("CRM MCP server stopped")?;
    Ok(())
}
