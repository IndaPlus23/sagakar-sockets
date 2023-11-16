use std::net::TcpStream;
use std::io::{Write, BufReader, BufRead};
use std::thread::{sleep, self};
use std::time::Duration;
use std::sync::mpsc::{self, Receiver};

const SLEEP_DURATION: Duration = Duration::from_millis(50);

// Escape sequences
const CLEAR: &str = "\x1B[2J";
const RESET_CURSOR: &str = "\x1B[1;1H";
const UP_ONE_LINE: &str = "\x1B[1F";

fn main() {
    // Get address
    print!("{}{}", CLEAR, RESET_CURSOR);
    println!("Enter server address:");
    let mut address = String::new();
    std::io::stdin().read_line(&mut address).expect("Failed reading user input");
    address = address.trim().to_owned();

    // Attempt connection
    let mut stream = match TcpStream::connect(&address) {
        Ok(stream) => {
            println!("Connected to server at {}", address);
            stream
        },
        Err(_) => {
            println!("Could not connect to server at address: {address}");
            println!("Shutting down...");
            std::process::exit(1);
        }
    };

    // Get username
    println!("Enter username:");
    let mut username = String::new();
    std::io::stdin().read_line(&mut username).expect("Failed reading user input");
    stream.write(username.as_bytes()).expect("Writing to server failed");
    print!("{}{}", CLEAR, RESET_CURSOR);

    let (sender, receiver) = mpsc::channel::<String>();
    let write_stream = stream.try_clone().expect("Failed copying TCP stream");
    let reader = BufReader::new(stream);

    // Create threads for communicating with server
    thread::spawn(move|| listen(reader));
    thread::spawn(move|| write_queued(write_stream, receiver));

    let mut outgoing_message = String::new();
    loop {
        outgoing_message.clear();
        std::io::stdin().read_line(&mut outgoing_message).expect("Failed reading user input");
        print!("{}", UP_ONE_LINE);
        // Quit client if user types "q"
        if outgoing_message.trim() == "q" {
            println!("Quitting...");
            std::process::exit(0);
        }

        sender.send(outgoing_message.clone()).expect("MPSC error");
    }
}

// Listen for messages from server and print them
fn listen(mut reader: BufReader<TcpStream>) {
    let mut incoming_message = String::new();
    // Listen for new messages and handle them
    loop {
        sleep(SLEEP_DURATION);
        // Clear possible old message
        incoming_message.clear();
        // Read incoming message and close stream if client has disconnected
        match reader.read_line(&mut incoming_message) {
            Ok(0) => {
                println!("Server disconnected, quitting...");
                std::process::exit(1);
            },
            Ok(_) => {}
            Err(_) => {
                println!("Server disconnected, quitting...");
                std::process::exit(1);
            },
        };
        // If no message has appeared, keep waiting
        if incoming_message.is_empty() {
            continue;
        }
        print!("{}", incoming_message);
    }   
}

// Write any queued messages to the server
fn write_queued(mut stream: TcpStream, receiver: Receiver<String>) {
    loop {
        if let Ok(outgoing_message) = receiver.try_recv() {
            match stream.write(outgoing_message.as_bytes()) {
                Ok(0) => {
                    println!("Server disconnected, quitting...");
                    std::process::exit(1);
                },
                Ok(_) => {}
                Err(_) => {
                    println!("Server disconnected, quitting...");
                    std::process::exit(1);
                },
            }
        }
        sleep(SLEEP_DURATION);
    }
}