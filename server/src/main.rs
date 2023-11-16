use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};
use std::thread::{self, sleep};
use std::time::{self, Duration};
use std::sync::mpsc::{self, Sender};

const SLEEP_DURATION: Duration = time::Duration::from_millis(100);

const MOTD: &str = "Welcome to my chat server!\n";

// ANSI escape sequences
const GREEN: &str = "\x1B[32m";
const YELLOW: &str = "\x1B[33m";
const RESET_COLOR: &str = "\x1B[0m";

fn main() {
    let listener = TcpListener::bind("0.0.0.0:5656").unwrap();
    let mut clients: Vec<(String, TcpStream)> = vec![];
    let (outgoing_sender, outgoing_receiver) = mpsc::channel::<(String, String)>(); // Outgoing message queue
    let (client_sender, client_receiver) = mpsc::channel::<(String, TcpStream)>(); // Client queue

    // Create a thread that accepts new connections. This is to allow blocking communications.
    thread::spawn(move|| accept_connections(listener, client_sender, outgoing_sender));

    loop {

        // If there is a new user, add to clients
        if let Ok((username, mut stream)) = client_receiver.try_recv() {
            if is_online(&username, &clients) {
                stream.shutdown(std::net::Shutdown::Both).expect("Failed shutting down stream");
                continue;
            }
            stream.write(MOTD.as_bytes()).expect("Failed writing to client");
            stream.write("Currently online:\n".as_bytes()).expect("Failed writing to client");
            stream.write(clients_to_string(&clients).as_bytes()).expect("Failed writing to client");
            clients.push((username, stream));
        }

        // If there is an outgoing message, broadcast it
        if let Ok((sender_name, body)) = outgoing_receiver.try_recv() {
            for (username, stream) in &mut clients {
                let mut color = YELLOW;
                if username == &sender_name {color = GREEN};
                let formatted_message = format!("{color}<{sender_name}> {body}{RESET_COLOR}\n");
                match stream.write(formatted_message.as_bytes()) {
                    Ok(_) => {},
                    Err(_) => {
                        username.clear();
                    },
                };
            }
            clients.retain(|(username, _stream)| !username.is_empty());
        }
        sleep(SLEEP_DURATION);
    }
}

fn accept_connections(listener: TcpListener, client_sender: Sender<(String, TcpStream)>, outgoing_sender: Sender<(String, String)>) {
    loop {
        if let Ok((stream, address)) = listener.accept() {
            let write_stream = stream.try_clone().expect("Failed cloning TCP stream"); // Copy stream to use for broadcasting
    
            let mut reader = BufReader::new(stream);
            // Read the first line from a new client as a username
            let mut username = String::new();
            reader.read_line(&mut username).expect("Malformed unicode");
            username = username.trim().to_owned();
    
            client_sender.send((username.clone(), write_stream)).expect("MPSC error");
            println!("Client {} at {} connected", username, address);
    
            // And create a thread to listen to incoming messages on the stream
            let thread_sender = outgoing_sender.clone();
            thread::spawn(move || listen(reader, thread_sender, username));
        }
    } 
}

fn clients_to_string(clients: &Vec<(String, TcpStream)>) -> String{
    let mut message = String::new();
    for (username, _stream) in clients {
        message += username;
        message += "\n";
    }
    return message;
}

// Checks if a user is currently online
fn is_online(username: &str, clients: &Vec<(String, TcpStream)>) -> bool {
    for (client_name, _stream) in clients {
        if username == client_name {
            return true
        }
    }
    return false
}

// Listen for incoming messages from a client. 
// Format any messages and put them on the outgoing message queue.
fn listen(mut reader: BufReader<TcpStream>, sender: Sender<(String, String)>, username: String) {
    let mut incoming_message = String::new();
    // Listen for new messages and handle them
    loop {
        sleep(SLEEP_DURATION);
        // Clear possible old message
        incoming_message.clear();
        // Read incoming message and close stream if client has disconnected
        match reader.read_line(&mut incoming_message) {
            Ok(0) => {
                println!("Client {} disconnected", username);
                break;
            },
            Ok(_) => {}
            Err(_) => {
                println!("Client {} disconnected", username);
                break;
            },
        };
        // If no message has appeared, keep waiting
        if incoming_message.is_empty() {
            continue;
        }
        let outgoing_message = incoming_message.trim().to_owned();
        println!("{}", outgoing_message);
        sender.send((username.clone(), outgoing_message)).expect("MPSC error");
    }   
}