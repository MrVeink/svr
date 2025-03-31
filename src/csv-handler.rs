// src/csv_handler.rs
use std::path::Path;
use csv::{ReaderBuilder, StringRecord};
use std::io::{Read, BufReader};
use std::fs::File;
use tokio::task;

use crate::data_types::TableData;

pub struct CSVHandler {}

impl CSVHandler {
    pub fn new() -> Self {
        CSVHandler {}
    }

    pub async fn read_csv<P: AsRef<Path> + Send + 'static>(&self, path: P) -> TableData {
        task::spawn_blocking(move || {
            let mut data = TableData::empty();

            // First check if file uses comma or semicolon as delimiter
            let delimiter = Self::detect_delimiter(&path);
            
            let file = match File::open(&path) {
                Ok(file) => file,
                Err(_) => return data,
            };
            
            let mut reader = ReaderBuilder::new()
                .delimiter(delimiter as u8)
                .flexible(true)
                .from_reader(file);

            // Process the CSV
            let headers: Vec<String> = match reader.headers() {
                Ok(headers) => headers.iter().map(String::from).collect(),
                Err(_) => return data,  // Return empty data if headers can't be read
            };

            // Find columns to hide and process headers
            let columns_to_hide = Self::get_columns_to_hide(&headers);
            let (processed_headers, visible_columns) = Self::process_headers(headers, &columns_to_hide);
            
            data.headers = processed_headers;
            
            // Read and process rows
            for result in reader.records() {
                match result {
                    Ok(record) => {
                        // Skip empty rows
                        if record.iter().all(|field| field.trim().is_empty()) {
                            continue;
                        }
                        
                        // Filter visible columns
                        let filtered_row: Vec<String> = record.iter()
                            .enumerate()
                            .filter(|(i, _)| i < &visible_columns.len() && visible_columns[*i])
                            .map(|(_, field)| field.to_string())
                            .collect();
                        
                        data.rows.push(filtered_row);
                    },
                    Err(_) => continue,
                }
            }

            data
        }).await.unwrap_or_else(|_| TableData::empty())
    }

    fn detect_delimiter<P: AsRef<Path>>(path: P) -> char {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return ',', // Default to comma if file can't be opened
        };
        
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        
        if reader.read_line(&mut first_line).is_ok() {
            if first_line.contains(';') {
                return ';';
            }
        }
        
        ','  // Default to comma
    }

    fn get_columns_to_hide(headers: &[String]) -> Vec<&str> {
        vec![
            "sport_id", "team_members", "team_name",
            "info", "result_code", "position_pre"
        ]
    }

    fn process_headers(
        headers: Vec<String>, 
        columns_to_hide: &[&str]
    ) -> (Vec<String>, Vec<bool>) {
        let mut processed_headers = Vec::new();
        let mut visible_columns = Vec::new();
        
        for header in headers {
            // Check if this column should be hidden
            let should_hide = columns_to_hide.iter()
                .any(|col| header.to_lowercase().contains(col));
            
            visible_columns.push(!should_hide);
            
            if !should_hide {
                // Apply header replacements
                let processed_header = Self::replace_header(&header);
                processed_headers.push(processed_header);
            }
        }
        
        (processed_headers, visible_columns)
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
