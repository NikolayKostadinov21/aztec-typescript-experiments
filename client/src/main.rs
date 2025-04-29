use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url; // Importing futures utils

#[tokio::main]
async fn main() {
    let url = Url::parse("ws://localhost:3002").unwrap();

    match connect_async(url).await {
        Ok((mut socket, _)) => {
            println!(" Connected to WebSocket server");

            // Send "set" action with value 214
            let set_request = json!({ "action": "set", "value": 214 }).to_string();
            socket.send(Message::Text(set_request)).await.unwrap();
            println!("Sent set request");

            // Wait for confirmation
            if let Some(Ok(Message::Text(response))) = socket.next().await {
                println!(" Response: {}", response);
            }

            // Delay for contract update (simulate waiting for transaction)
            sleep(Duration::from_secs(1)).await;

            // Send "get" action to retrieve the value
            let get_request = json!({ "action": "get" }).to_string();
            socket.send(Message::Text(get_request)).await.unwrap();
            println!("Sent get request");

            // Wait for value response
            if let Some(Ok(Message::Text(response))) = socket.next().await {
                println!("Retrieved Value: {}", response);
            }
        }
        Err(e) => {
            eprintln!("WebSocket connection failed: {}", e);
        }
    }
}

// while let Some(msg) = socket.next().await {
//     match msg {
//         Ok(Message::Text(text)) => println!(" WebSocket Response: {}", text),
//         Ok(_) => println!("Received non-text WebSocket message"),
//         Err(e) => println!(" Error receiving WebSocket message: {}", e),
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
}
