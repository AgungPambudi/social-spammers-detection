use std::fs;
use std::io::{self, Write};
use std::path::Path;

use google_youtube3::api::CommentThread;
use google_youtube3::{YouTube, oauth2, hyper, hyper_rustls};
use oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
use tokio;

// Define the asynchronous entry point using Tokio's runtime
#[tokio::main]
async fn main() {
    // Load environment variables from `.env` file
    dotenvy::dotenv().ok();

    // Read YouTube video ID from environment variable
    let target_video_id = std::env::var("YOUTUBE_VIDEO_ID").expect("Missing YOUTUBE_VIDEO_ID");

    // Authenticate the user with OAuth and proceed if successful
    match setup_authentication().await {
        Ok(authenticator) => {
            // Build a YouTube API client with authenticated HTTP client
            let youtube_client = YouTube::new(
                hyper::Client::builder()
                    .build(
                        hyper_rustls::HttpsConnectorBuilder::new()
                            .with_native_roots() // Use system certificate store
                            .https_or_http()     // Allow both HTTP and HTTPS
                            .enable_http1()      // Enable HTTP/1
                            .build()
                    ),
                authenticator,
            );

            // Fetch potentially spammy comments from the video
            match collect_flagged_comments(&youtube_client, &target_video_id).await {
                Ok(suspicious_comment_ids) => {
                    // If any suspicious comments found, delete them
                    if !suspicious_comment_ids.is_empty() {
                        println!("Found {} spam comments. Deleting...", suspicious_comment_ids.len());
                        purge_comments(&youtube_client, &suspicious_comment_ids).await;
                    } else {
                        println!("No spam comments found.");
                    }
                }
                // Handle error in fetching comments
                Err(e) => eprintln!("Failed to collect comments: {}", e),
            }
        }
        // Handle error in authentication
        Err(e) => eprintln!("Authorization failed: {}", e),
    }
}

// Perform interactive OAuth authentication and return an authenticator
async fn setup_authentication() -> Result<oauth2::authenticator::Authenticator<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>, Box<dyn std::error::Error>> {
    // Load OAuth client secret from `credentials.json`
    let credentials = oauth2::read_application_secret("credentials.json").await?;

    // Path to save the OAuth token
    let token_storage_path = "token.json";

    // Build the OAuth authenticator with the credentials and save token to disk
    let authenticator = InstalledFlowAuthenticator::builder(credentials, InstalledFlowReturnMethod::Interactive)
        .persist_tokens_to_disk(token_storage_path)
        .build()
        .await?;

    Ok(authenticator)
}

// Fetch comments from a YouTube video and return IDs of those suspected to be spam
async fn collect_flagged_comments(
    youtube_api: &YouTube<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>, 
    video_id: &str
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Perform API call to list comment threads on the video
    let response = youtube_api.comment_threads().list(&vec!["snippet".to_string()])
        .video_id(video_id)
        .max_results(100) // Limit to 100 comments
        .doit()
        .await?;

    // Store comment IDs suspected to be spam
    let mut flagged_comment_ids = Vec::new();

    // Iterate through comment thread items (if any)
    if let Some(thread_list) = response.1.items {
        for thread in thread_list {
            if let Some(thread_snippet) = thread.snippet {
                if let Some(top_level_comment) = thread_snippet.top_level_comment {
                    if let Some(comment_data) = top_level_comment.snippet {
                        // Extract the comment text and ID
                        let content = comment_data.text_display.unwrap_or_default();
                        let comment_identifier = thread.id.unwrap_or_default();

                        println!("Analyzing comment: \"{}\"", content);

                        // If the comment contains unusual Unicode, flag it
                        if contains_unicode_abuse(&content) {
                            println!("🚨 Spam detected: \"{}\"", content);
                            flagged_comment_ids.push(comment_identifier);
                        }
                    }
                }
            }
        }
    }

    Ok(flagged_comment_ids)
}

// Delete comments by ID using the YouTube API
async fn purge_comments(
    youtube_api: &YouTube<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>, 
    comment_ids: &[String]
) {
    // Loop through and delete each flagged comment
    for comment_id in comment_ids {
        match youtube_api.comments().delete(comment_id).doit().await {
            Ok(_) => println!("Successfully removed comment: {}", comment_id),
            Err(err) => eprintln!("Error removing comment {}: {}", comment_id, err),
        }
    }
}

// Check if the text has visually similar but Unicode-abusive characters
fn contains_unicode_abuse(content: &str) -> bool {
    // Normalize the text and compare with original; difference implies Unicode obfuscation
    let normalized_content = content.nfkd().collect::<String>();
    content != normalized_content
}

// Trait import required for Unicode normalization operations like .nfkd()
use unicode_normalization::UnicodeNormalization;