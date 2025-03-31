// src/cloud_handler.rs
use google_sheets4::{api::ValueRange, Sheets};
use std::error::Error;
use tokio::task;
use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};
use serde_json::from_str;
use std::fs::File;
use std::io::Read;

use crate::data_types::TableData;

pub struct CloudHandler {
    credentials_path: String,
}

impl CloudHandler {
    pub fn new() -> Self {
        CloudHandler {
            credentials_path: "credentials.json".to_string(),
        }
    }

    pub async fn fetch_data(&self, spreadsheet_url: &str, sheet_name: &str) -> Result<TableData, Box<dyn Error>> {
        let spreadsheet_id = self.extract_spreadsheet_id(spreadsheet_url)?;
        let sheet = if sheet_name.is_empty() { "Sheet1" } else { sheet_name };
        
        // Authenticate with Google Sheets API
        let sheets = self.authenticate().await?;
        
        // Fetch data from Google Sheets
        let range = format!("{}!A:Z", sheet);
        let response = sheets.spreadsheets().values_get(spreadsheet_id, &range).await?;
        
        // Process the data
        self.process_data(response).await
    }

    async fn authenticate(&self) -> Result<Sheets, Box<dyn Error>> {
        // Load service account key from file
        let mut json = String::new();
        File::open(&self.credentials_path)?.read_to_string(&mut json)?;
        
        let service_account_key: ServiceAccountKey = from_str(&json)?;
        
        // Create authenticator
        let auth = ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await?;
        
        // Create an authenticated Sheets client
        let sheets = Sheets::new(
            reqwest::Client::builder()
                .build()?,
            auth,
        );
        
        Ok(sheets)
    }

    fn extract_spreadsheet_id(&self, url: &str) -> Result<&str, Box<dyn Error>> {
        // Extract spreadsheet ID from URL
        // URLs typically look like: https://docs.google.com/spreadsheets/d/[SPREADSHEET_ID]/edit
        let parts: Vec<&str> = url.split('/').collect();
        
        for (i, part) in parts.iter().enumerate() {
            if *part == "d" && i + 1 < parts.len() {
                return Ok(parts[i+1]);
            }
        }
        
        Err("Invalid spreadsheet URL".into())
    }

    async fn process_data(&self, response: ValueRange) -> Result<TableData, Box<dyn Error>> {
        // Process the data from Google Sheets
        task::spawn_blocking(move || {
            let mut data = TableData::empty();
            
            if let Some(values) = response.values {
                if values.is_empty() {
                    return data;
                }
                
                // Find the first row where the first cell contains "category" (case-insensitive)
                let mut start_index = 0;
                for (i, row) in values.iter().enumerate() {
                    if !row.is_empty() && row[0].to_string().to_lowercase() == "category" {
                        start_index = i;
                        break;
                    }
                }
                
                // Extract data from the category row onward
                let relevant_data = &values[start_index..];
                if relevant_data.is_empty() {
                    return data;
                }
                
                // Columns to hide
                let columns_to_hide = vec![
                    "sport_id", "team_members", "team_name",
                    "info", "result_code", "position_pre"
                ];
                
                // Process headers
                let headers = &relevant_data[0];
                let mut visible_columns = Vec::new();
                let mut processed_headers = Vec::new();
                
                for (i, header) in headers.iter().enumerate() {
                    let header_str = header.to_string();
                    
                    // Check if this column should be hidden
                    let should_hide = columns_to_hide.iter()
                        .any(|col| header_str.to_lowercase().contains(col));
                    
                    visible_columns.push(!should_hide);
                    
                    if !should_hide {
                        // Apply header replacements
                        let processed_header = Self::replace_header(&header_str);
                        processed_headers.push(processed_header);
                    }
                }
                
                data.headers = processed_headers;
                
                // Process rows
                for row in relevant_data.iter().skip(1) {
                    // Skip empty rows
                    if row.is_empty() || row.iter().all(|cell| cell.as_str().map_or(true, |s| s.trim().is_empty())) {
                        continue;
                    }
                    
                    // Filter visible columns
                    let mut filtered_row = Vec::new();
                    for (i, cell) in row.iter().enumerate() {
                        if i < visible_columns.len() && visible_columns[i] {
                            filtered_row.push(cell.to_string());
                        }
                    }
                    
                    data.rows.push(filtered_row);
                }
            }
            
            data
        }).await.unwrap_or_else(|_| TableData::empty())
    }

    fn replace_header(header: &str) -> String {
        let header_lower = header.to_lowercase();
        
        // Header replacements mapping
        let replacements = [
            ("category", "Series"),
            ("first_name", "Name"),
            ("last_name", "Surname"),
            ("organization", "Club"),
            ("napat", "X"),
            ("result", "Result"),
            ("posit.", "Rank")
        ];
        
        // First check for part-X and psum-X patterns
        if header_lower.contains("part-") {
            if let Some(part_num) = header.split('-').nth(1) {
                return format!("S{}", part_num);
            }
        } else if header_lower.contains("psum-") {
            if let Some(part_num) = header.split('-').nth(1) {
                return format!("P{}", part_num);
            }
        }
        
        // Then check other replacements
        for (original, replacement) in replacements.iter() {
            if header_lower.contains(original) {
                return replacement.to_string();
            }
        }
        
        header.to_string()
    }
}
